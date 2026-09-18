package pilot

import (
	"bytes"
	"crypto/ed25519"
	"encoding/base64"
	"encoding/json"
	"net/http/httptest"
	"sync"
	"testing"
	"time"
)

func request(s *Server, method, path, token string, body any) (int, map[string]any) {
	b, _ := json.Marshal(body)
	r := httptest.NewRequest(method, path, bytes.NewReader(b))
	if token != "" {
		r.Header.Set("Authorization", "Bearer "+token)
	}
	w := httptest.NewRecorder()
	s.Handler().ServeHTTP(w, r)
	var result map[string]any
	_ = json.Unmarshal(w.Body.Bytes(), &result)
	return w.Code, result
}
func fixture(t *testing.T) (*Server, string, map[string]any) {
	t.Helper()
	admin := randomToken()
	s, e := New(admin)
	if e != nil {
		t.Fatal(e)
	}
	code, r := request(s, "POST", "/pilot/create", admin, nil)
	if code != 201 {
		t.Fatal(code)
	}
	return s, admin, r
}
func desc(kind string) map[string]string {
	return map[string]string{"type": kind, "sdp": "v=0\r\na=fingerprint:sha-256 01:02:03:04:05:06:07:08:09:0A:0B:0C:0D:0E:0F:10:11:12:13:14:15:16:17:18:19:1A:1B:1C:1D:1E:1F:20\r\n"}
}
func TestInvitationAtomicAndConsentRequired(t *testing.T) {
	s, _, r := fixture(t)
	host := r["token"].(string)
	var wg sync.WaitGroup
	var mu sync.Mutex
	wins := 0
	viewer := ""
	for i := 0; i < 20; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			code, v := request(s, "POST", "/pilot/join", "", map[string]string{"invite": r["invite"].(string), "name": "Windows"})
			if code == 200 {
				mu.Lock()
				wins++
				viewer = v["token"].(string)
				mu.Unlock()
			}
		}()
	}
	wg.Wait()
	if wins != 1 {
		t.Fatal(wins)
	}
	for _, token := range []string{host, viewer} {
		code, _ := request(s, "POST", "/pilot/action", token, map[string]any{"action": "offer", "description": desc("offer")})
		if code != 409 {
			t.Fatal("offer before consent", code)
		}
	}
	code, _ := request(s, "POST", "/pilot/action", viewer, map[string]string{"action": "accept"})
	if code != 409 {
		t.Fatal("viewer consent", code)
	}
	request(s, "POST", "/pilot/action", host, map[string]string{"action": "accept"})
	request(s, "POST", "/pilot/action", host, map[string]any{"action": "offer", "description": desc("offer")})
	code, result := request(s, "POST", "/pilot/action", viewer, map[string]any{"action": "answer", "description": desc("answer")})
	if code != 200 || result["state"] != "active" {
		t.Fatal(result)
	}
	g := result["grant"].(map[string]any)
	payload, _ := base64.StdEncoding.DecodeString(g["payload"].(string))
	sig, _ := base64.StdEncoding.DecodeString(g["signature"].(string))
	if !ed25519.Verify(s.key.Public().(ed25519.PublicKey), payload, sig) {
		t.Fatal("grant signature")
	}
	request(s, "POST", "/pilot/action", host, map[string]string{"action": "end"})
	_, result = request(s, "GET", "/pilot/room", viewer, nil)
	if result["state"] != "ended" || result["grant"] != nil || result["offer"] != nil {
		t.Fatal("revocation", result)
	}
}
func TestAuthExpiryAndOrigin(t *testing.T) {
	s, _, r := fixture(t)
	if code, _ := request(s, "POST", "/pilot/create", "wrong", nil); code != 401 {
		t.Fatal(code)
	}
	now := s.now()
	s.now = func() time.Time { return now.Add(31 * time.Minute) }
	if code, _ := request(s, "GET", "/pilot/room", r["token"].(string), nil); code != 401 {
		t.Fatal(code)
	}
	req := httptest.NewRequest("GET", "/pilot/room", nil)
	req.Header.Set("Origin", "https://evil.example")
	w := httptest.NewRecorder()
	s.Handler().ServeHTTP(w, req)
	if w.Code != 403 {
		t.Fatal(w.Code)
	}
}
func TestHeartbeatClosesSession(t *testing.T) {
	s, _, r := fixture(t)
	now := s.now()
	s.now = func() time.Time { return now.Add(61 * time.Second) }
	_, result := request(s, "GET", "/pilot/room", r["token"].(string), nil)
	if result["state"] != "ended" {
		t.Fatal(result)
	}
}
func TestGuestHostsOptInAndConsent(t *testing.T) {
	s, _, existing := fixture(t)
	if code, _ := request(s, "POST", "/pilot/create-guest", "", map[string]any{}); code != 403 {
		t.Fatal("guest hosts must be disabled by default", code)
	}
	s.AllowGuestHosts = true
	code, host := request(s, "POST", "/pilot/create-guest", "", map[string]any{})
	if code != 201 {
		t.Fatal(code, host)
	}
	token := host["token"].(string)
	if code, _ = request(s, "POST", "/pilot/create", token, nil); code != 401 {
		t.Fatal("guest gained administration access")
	}
	if code, _ = request(s, "POST", "/pilot/action", token, map[string]any{"action": "offer", "description": desc("offer")}); code != 409 {
		t.Fatal("screen offer allowed before consent")
	}
	_, viewer := request(s, "POST", "/pilot/join", "", map[string]string{"invite": host["invite"].(string), "name": "Mac viewer"})
	if code, _ = request(s, "POST", "/pilot/action", viewer["token"].(string), map[string]string{"action": "accept"}); code != 409 {
		t.Fatal("viewer accepted for host")
	}
	if code, _ = request(s, "POST", "/pilot/action", token, map[string]string{"action": "accept"}); code != 200 {
		t.Fatal("Windows host consent failed")
	}
	_, other := request(s, "GET", "/pilot/room", existing["token"].(string), nil)
	if other["state"] != "waiting" {
		t.Fatal("guest affected an existing room")
	}
	if code, _ = request(s, "POST", "/pilot/create-guest", "", map[string]any{"device": "another-device"}); code != 400 {
		t.Fatal("unknown fields accepted")
	}
}
func TestGuestAdmissionIsBoundedUnderRace(t *testing.T) {
	s, _, _ := fixture(t)
	s.AllowGuestHosts = true
	var wg sync.WaitGroup
	var mu sync.Mutex
	wins := 0
	for i := 0; i < 20; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			code, _ := request(s, "POST", "/pilot/create-guest", "", map[string]any{})
			if code == 201 {
				mu.Lock()
				wins++
				mu.Unlock()
			} else if code != 429 {
				t.Errorf("unexpected response %d", code)
			}
		}()
	}
	wg.Wait()
	if wins != 6 {
		t.Fatalf("created %d rooms; budget must allow six", wins)
	}
	if code, _ := request(s, "POST", "/pilot/create-guest", "", map[string]any{}); code != 429 {
		t.Fatal("rate limit not enforced")
	}
	now := s.now()
	s.now = func() time.Time { return now.Add(61 * time.Second) }
	if code, _ := request(s, "POST", "/pilot/create-guest", "", map[string]any{}); code != 201 {
		t.Fatal("expired room capacity not recovered", code)
	}
}
