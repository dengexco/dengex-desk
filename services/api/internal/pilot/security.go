package pilot

import (
	"bytes"
	"io"
	"net/http"
	"time"
)

// Limits are deliberately global: forwarding headers are not trusted identities.
// Authenticated heartbeat/stop requests never spend the failed-auth budget.
type budget struct {
	start time.Time
	used  int
}

func (b *budget) allow(now time.Time, limit int) bool {
	if b.start.IsZero() || now.Sub(b.start) >= time.Minute {
		b.start, b.used = now, 0
	}
	if b.used >= limit {
		return false
	}
	b.used++
	return true
}

type SecurityEvent struct {
	At   int64  `json:"at"`
	Kind string `json:"kind"`
}

// Only fixed event kinds and timestamps: no tokens, IPs, names, SDP or images.
func (s *Server) record(room *Room, kind string) {
	event := SecurityEvent{At: s.now().Unix(), Kind: kind}
	s.events = append(s.events, event)
	if len(s.events) > 128 {
		s.events = s.events[len(s.events)-128:]
	}
	if room != nil {
		room.Events = append(room.Events, event)
		if len(room.Events) > 16 {
			room.Events = room.Events[len(room.Events)-16:]
		}
	}
}

func (s *Server) endRoom(room *Room, reason string) {
	if room.State == "ended" {
		return
	}
	room.State, room.Offer, room.Answer = "ended", nil, nil
	room.InviteHash = [32]byte{}
	s.record(room, reason)
}

func limited(w http.ResponseWriter, reason string) {
	w.Header().Set("Retry-After", "60")
	bad(w, http.StatusTooManyRequests, reason)
}

func (s *Server) security(w http.ResponseWriter, r *http.Request) {
	if !same(tokenHash(r), s.createHash) {
		bad(w, http.StatusUnauthorized, "unauthorized")
		return
	}
	reply(w, http.StatusOK, map[string]any{"events": s.events, "retention": "memory_only_last_128"})
}

// JSON is read before taking the room lock, and responses are copied after it
// is released. A slow upload/download cannot hold every active room's lock.
type bufferedResponse struct {
	header http.Header
	status int
	bytes.Buffer
}

func (w *bufferedResponse) Header() http.Header    { return w.header }
func (w *bufferedResponse) WriteHeader(status int) { w.status = status }

func (s *Server) boundedBody(w http.ResponseWriter, r *http.Request) bool {
	if r.Method != http.MethodPost {
		return true
	}
	select {
	case s.uploads <- struct{}{}:
		defer func() { <-s.uploads }()
	default:
		w.Header().Set("Retry-After", "1")
		bad(w, http.StatusTooManyRequests, "server_busy")
		return false
	}
	data, err := io.ReadAll(http.MaxBytesReader(w, r.Body, maxBody))
	_ = r.Body.Close()
	if err != nil {
		bad(w, http.StatusRequestEntityTooLarge, "invalid_request")
		return false
	}
	r.Body = io.NopCloser(bytes.NewReader(data))
	return true
}

func (s *Server) failedAuthLimited(w http.ResponseWriter, r *http.Request) bool {
	admin := r.URL.Path == "/pilot/create" || r.URL.Path == "/pilot/security"
	if !admin && r.URL.Path != "/pilot/room" && r.URL.Path != "/pilot/action" {
		return false
	}
	h := tokenHash(r)
	valid := admin && same(h, s.createHash)
	if !admin && h != [32]byte{} {
		for _, room := range s.rooms {
			valid = valid || same(h, room.HostHash) || same(h, room.ViewerHash)
		}
	}
	if valid {
		return false
	}
	if !s.failedAuth.allow(s.now(), 20) {
		limited(w, "authentication_rate_limited")
		return true
	}
	s.record(nil, "authentication_rejected")
	return false
}
