// Package pilot implements an isolated, attended, view-only two-device pilot.
// It is not the tenant/OIDC product API. Optional guest hosts can create only
// their own attended, view-only rooms; they gain no access to existing devices.
package pilot

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

const maxBody = 96 << 10

type Room struct {
	ID                               string
	HostHash, InviteHash, ViewerHash [32]byte
	ViewerName                       string
	State                            string
	Offer, Answer                    json.RawMessage
	Expires                          time.Time
	HostSeen, ViewerSeen             time.Time
}
type Server struct {
	mu              sync.Mutex
	rooms           map[string]*Room
	createHash      [32]byte
	key             ed25519.PrivateKey
	now             func() time.Time
	AllowGuestHosts bool // Explicit deployment opt-in; configure before serving.
	guestWindow     time.Time
	guestCreates    int
}

func New(createToken string) (*Server, error) {
	if len(createToken) < 40 {
		return nil, errors.New("pilot creation token requires at least 32 random bytes")
	}
	_, key, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}
	return &Server{rooms: make(map[string]*Room), createHash: sha256.Sum256([]byte(createToken)), key: key, now: time.Now}, nil
}
func randomToken() string {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		panic(err)
	}
	return base64.RawURLEncoding.EncodeToString(b)
}
func tokenHash(r *http.Request) [32]byte {
	return sha256.Sum256([]byte(strings.TrimPrefix(r.Header.Get("Authorization"), "Bearer ")))
}
func same(a, b [32]byte) bool { return subtle.ConstantTimeCompare(a[:], b[:]) == 1 }
func reply(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}
func bad(w http.ResponseWriter, status int, reason string) {
	reply(w, status, map[string]string{"error": reason})
}
func read(r *http.Request, w http.ResponseWriter, v any) bool {
	r.Body = http.MaxBytesReader(w, r.Body, maxBody)
	d := json.NewDecoder(r.Body)
	d.DisallowUnknownFields()
	if d.Decode(v) != nil {
		bad(w, 400, "invalid_request")
		return false
	}
	if d.Decode(new(any)) != io.EOF {
		bad(w, 400, "invalid_request")
		return false
	}
	return true
}
func (s *Server) Handler() http.Handler {
	m := http.NewServeMux()
	m.HandleFunc("POST /pilot/create", s.create)
	m.HandleFunc("POST /pilot/create-guest", s.createGuest)
	m.HandleFunc("POST /pilot/join", s.join)
	m.HandleFunc("GET /pilot/room", s.room)
	m.HandleFunc("POST /pilot/action", s.action)
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		// Native clients use bearer headers, never cookies. Browser cross-origin access
		// is unnecessary; the local bundled UI calls through the native client.
		if r.Header.Get("Origin") != "" {
			bad(w, 403, "native_client_required")
			return
		}
		if r.URL.RawQuery != "" {
			bad(w, 400, "query_parameters_forbidden")
			return
		}
		s.mu.Lock()
		defer s.mu.Unlock()
		for id, room := range s.rooms {
			if s.now().After(room.Expires) {
				delete(s.rooms, id)
				continue
			}
			if room.State != "ended" && ((!room.HostSeen.IsZero() && s.now().Sub(room.HostSeen) > 60*time.Second) || (!room.ViewerSeen.IsZero() && s.now().Sub(room.ViewerSeen) > 60*time.Second)) {
				room.State = "ended"
				room.Offer = nil
				room.Answer = nil
			}
		}
		m.ServeHTTP(w, r)
	})
}
func (s *Server) create(w http.ResponseWriter, r *http.Request) {
	if !same(tokenHash(r), s.createHash) {
		bad(w, 401, "unauthorized")
		return
	}
	s.issueRoom(w)
}
func (s *Server) createGuest(w http.ResponseWriter, r *http.Request) {
	if !s.AllowGuestHosts {
		bad(w, 403, "guest_hosts_disabled")
		return
	}
	var request struct{}
	if !read(r, w, &request) {
		return
	}
	// A global bounded admission budget also covers clients behind the local
	// Cloudflare proxy. No client-controlled forwarding header bypasses this.
	if s.guestWindow.IsZero() || s.now().Sub(s.guestWindow) >= time.Minute {
		s.guestWindow = s.now()
		s.guestCreates = 0
	}
	if s.guestCreates >= 6 {
		bad(w, 429, "creation_rate_limited")
		return
	}
	s.guestCreates++
	s.issueRoom(w)
}
func (s *Server) issueRoom(w http.ResponseWriter) {
	for id, room := range s.rooms {
		if room.State == "ended" {
			delete(s.rooms, id)
		}
	}
	if len(s.rooms) >= 8 {
		bad(w, 429, "pilot_capacity")
		return
	}
	host, invite, id := randomToken(), randomToken(), randomToken()
	room := &Room{ID: id, HostHash: sha256.Sum256([]byte(host)), InviteHash: sha256.Sum256([]byte(invite)), State: "waiting", Expires: s.now().Add(30 * time.Minute), HostSeen: s.now()}
	s.rooms[id] = room
	reply(w, 201, map[string]any{"token": host, "invite": invite, "room": s.snapshot(room, "host"), "authorityKey": base64.StdEncoding.EncodeToString(s.key.Public().(ed25519.PublicKey))})
}
func (s *Server) join(w http.ResponseWriter, r *http.Request) {
	var req struct {
		Invite string `json:"invite"`
		Name   string `json:"name"`
	}
	if !read(r, w, &req) {
		return
	}
	if len(req.Invite) != 43 || len(req.Name) < 1 || len(req.Name) > 64 || strings.ContainsAny(req.Name, "\r\n\x00") {
		bad(w, 400, "invalid_invitation")
		return
	}
	h := sha256.Sum256([]byte(req.Invite))
	for _, room := range s.rooms {
		if same(h, room.InviteHash) && room.State == "waiting" {
			token := randomToken()
			room.ViewerHash = sha256.Sum256([]byte(token))
			room.InviteHash = [32]byte{}
			room.ViewerName = req.Name
			room.State = "awaiting_consent"
			room.ViewerSeen = s.now()
			reply(w, 200, map[string]any{"token": token, "room": s.snapshot(room, "viewer"), "authorityKey": base64.StdEncoding.EncodeToString(s.key.Public().(ed25519.PublicKey))})
			return
		}
	}
	bad(w, 401, "invitation_invalid_expired_or_used")
}
func (s *Server) authenticate(r *http.Request) (*Room, string) {
	h := tokenHash(r)
	for _, room := range s.rooms {
		if same(h, room.HostHash) {
			room.HostSeen = s.now()
			return room, "host"
		}
		if same(h, room.ViewerHash) {
			room.ViewerSeen = s.now()
			return room, "viewer"
		}
	}
	return nil, ""
}
func (s *Server) room(w http.ResponseWriter, r *http.Request) {
	room, role := s.authenticate(r)
	if room == nil {
		bad(w, 401, "session_expired")
		return
	}
	reply(w, 200, s.snapshot(room, role))
}
func fingerprint(description json.RawMessage) (string, error) {
	var d struct {
		Type string `json:"type"`
		SDP  string `json:"sdp"`
	}
	if json.Unmarshal(description, &d) != nil || len(d.SDP) < 20 || len(d.SDP) > 64<<10 {
		return "", errors.New("invalid_sdp")
	}
	found := ""
	for _, line := range strings.Split(d.SDP, "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "a=fingerprint:") {
			p := strings.Fields(strings.TrimPrefix(line, "a=fingerprint:"))
			if len(p) != 2 || p[0] != "sha-256" {
				return "", errors.New("unsupported_fingerprint")
			}
			b, err := hex.DecodeString(strings.ReplaceAll(p[1], ":", ""))
			if err != nil || len(b) != 32 {
				return "", errors.New("invalid_fingerprint")
			}
			v := strings.ToUpper(p[1])
			if found != "" && found != v {
				return "", errors.New("multiple_fingerprints")
			}
			found = v
		}
	}
	if found == "" {
		return "", errors.New("missing_fingerprint")
	}
	return found, nil
}
func (s *Server) action(w http.ResponseWriter, r *http.Request) {
	room, role := s.authenticate(r)
	if room == nil {
		bad(w, 401, "session_expired")
		return
	}
	var req struct {
		Action      string          `json:"action"`
		Description json.RawMessage `json:"description,omitempty"`
	}
	if !read(r, w, &req) {
		return
	}
	switch req.Action {
	case "end":
		room.State = "ended"
		room.Offer = nil
		room.Answer = nil
	case "accept":
		if role != "host" || room.State != "awaiting_consent" {
			bad(w, 409, "invalid_transition")
			return
		}
		room.State = "negotiating"
	case "offer", "answer":
		host := req.Action == "offer"
		if (host && (role != "host" || room.State != "negotiating" || room.Offer != nil)) || (!host && (role != "viewer" || room.State != "negotiating" || room.Offer == nil || room.Answer != nil)) {
			bad(w, 409, "invalid_transition")
			return
		}
		var desc struct {
			Type string `json:"type"`
		}
		_ = json.Unmarshal(req.Description, &desc)
		if desc.Type != req.Action {
			bad(w, 400, "sdp_type_mismatch")
			return
		}
		if _, err := fingerprint(req.Description); err != nil {
			bad(w, 400, err.Error())
			return
		}
		if host {
			room.Offer = req.Description
		} else {
			room.Answer = req.Description
			room.State = "active"
		}
	default:
		bad(w, 400, "unknown_action")
		return
	}
	reply(w, 200, s.snapshot(room, role))
}
func (s *Server) snapshot(room *Room, role string) map[string]any {
	result := map[string]any{"id": room.ID, "role": role, "state": room.State, "viewerName": room.ViewerName, "expiresAt": room.Expires.Unix(), "capabilities": []string{"view_screen"}, "accountVerified": false}
	if role == "viewer" && room.Offer != nil {
		result["offer"] = room.Offer
	}
	if role == "host" && room.Answer != nil {
		result["answer"] = room.Answer
	}
	if room.State == "active" {
		local, _ := fingerprint(room.Offer)
		peer, _ := fingerprint(room.Answer)
		if role == "viewer" {
			local, peer = peer, local
		}
		expires := s.now().Add(45 * time.Second)
		if expires.After(room.Expires) {
			expires = room.Expires
		}
		payload, _ := json.Marshal(map[string]any{"v": 1, "tenant_id": "attended-pilot", "device_id": room.ID, "user_id": "invited-viewer", "session_id": room.ID, "jti": randomToken(), "policy_version": 1, "input_epoch": 1, "issued_at": s.now().Unix(), "expires_at": expires.Unix(), "peer_fingerprint": peer, "local_fingerprint": local, "capabilities": []string{"view_screen"}})
		result["grant"] = map[string]string{"payload": base64.StdEncoding.EncodeToString(payload), "signature": base64.StdEncoding.EncodeToString(ed25519.Sign(s.key, payload))}
	}
	return result
}
