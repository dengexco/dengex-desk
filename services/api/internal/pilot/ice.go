package pilot

import (
	"crypto/hmac"
	"crypto/sha1" // coturn REST credentials use HMAC-SHA1, not a password hash.
	"encoding/base64"
	"errors"
	"fmt"
	"net"
	"net/url"
	"strconv"
	"strings"
	"time"
)

type iceServer struct {
	URLs       []string `json:"urls"`
	Username   string   `json:"username,omitempty"`
	Credential string   `json:"credential,omitempty"`
}

// ConfigureTURN must be called before Handler is served. The shared secret is
// never returned to clients. The native transport supports TURN over UDP only.
func (s *Server) ConfigureTURN(address, secret string, relayOnly bool) error {
	if address == "" {
		if secret != "" || relayOnly {
			return errors.New("TURN URL required with secret or relay-only mode")
		}
		return nil
	}
	u, err := url.Parse(address)
	if err != nil || u.Scheme != "turn" || u.Opaque == "" || u.Fragment != "" || u.RawQuery != "transport=udp" {
		return errors.New("TURN URL must be turn:host:port?transport=udp")
	}
	host, port, err := net.SplitHostPort(u.Opaque)
	n, pe := strconv.Atoi(port)
	if err != nil || pe != nil || n < 1 || n > 65535 || host == "" || strings.ContainsAny(host, "/@?# \t\r\n") {
		return errors.New("invalid TURN host or port")
	}
	if len(secret) < 32 || strings.ContainsAny(secret, "\r\n") {
		return errors.New("TURN shared secret requires at least 32 characters")
	}
	s.turnURL, s.turnSecret, s.relayOnly = address, secret, relayOnly
	return nil
}

func (s *Server) ice(room *Room, role string) map[string]any {
	servers := []iceServer{{URLs: []string{"stun:stun.cloudflare.com:3478"}}}
	policy := "all"
	if s.turnURL != "" {
		// One credential per role and room. Expiration covers the bounded session plus
		// one minute for teardown. Ending a room revokes media grants, not TURN creds.
		username := fmt.Sprintf("%d:%s:%s", room.Expires.Add(time.Minute).Unix(), room.ID, role)
		mac := hmac.New(sha1.New, []byte(s.turnSecret))
		_, _ = mac.Write([]byte(username))
		servers = append(servers, iceServer{URLs: []string{s.turnURL}, Username: username, Credential: base64.StdEncoding.EncodeToString(mac.Sum(nil))})
		if s.relayOnly {
			policy = "relay"
		}
	}
	return map[string]any{"iceServers": servers, "iceTransportPolicy": policy}
}
