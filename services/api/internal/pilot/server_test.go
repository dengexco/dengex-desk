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
