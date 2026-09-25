package pilot

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestFailedAttemptsDoNotBlockAuthenticatedStop(t *testing.T) {
	s, _, host := fixture(t)
	for i := 0; i < 25; i++ {
		code, _ := request(s, "GET", "/pilot/room", "wrong", nil)
		want := 401
		if i >= 20 {
			want = 429
		}
		if code != want {
			t.Fatalf("attempt %d: %d", i, code)
		}
	}
	if code, _ := request(s, "POST", "/pilot/action", host["token"].(string), map[string]string{"action": "end"}); code != 200 {
		t.Fatal("security budget prevented stop", code)
	}
}

func TestJoinBudgetCannotBeBypassedWithForwardingHeaders(t *testing.T) {
	s, _, host := fixture(t)
	var wg sync.WaitGroup
	for i := 0; i < 40; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			r := httptest.NewRequest("POST", "/pilot/join", strings.NewReader(`{"invite":"invalid","name":"test"}`))
			r.Header.Set("X-Forwarded-For", strings.Repeat("1", i+1))
			w := httptest.NewRecorder()
			s.Handler().ServeHTTP(w, r)
			if w.Code != 400 && w.Code != 429 {
				t.Errorf("unexpected %d", w.Code)
			}
		}(i)
	}
	wg.Wait()
	if s.joins.used != 30 {
		t.Fatal(s.joins.used)
	}
	if code, _ := request(s, "POST", "/pilot/join", "", map[string]string{"invite": host["invite"].(string), "name": "test"}); code != 429 {
		t.Fatal(code)
	}
	now := s.now()
	s.now = func() time.Time { return now.Add(30 * time.Second) }
	request(s, "GET", "/pilot/room", host["token"].(string), nil)
	s.now = func() time.Time { return now.Add(time.Minute) }
	if code, _ := request(s, "POST", "/pilot/join", "", map[string]string{"invite": host["invite"].(string), "name": "test"}); code != 200 {
		t.Fatal("budget did not recover", code)
	}
}

func TestInvitationExpiresEvenWithHostHeartbeats(t *testing.T) {
	s, _, host := fixture(t)
	now := s.now()
	for seconds := 30; seconds <= 300; seconds += 30 {
		at := now.Add(time.Duration(seconds) * time.Second)
		s.now = func() time.Time { return at }
		_, room := request(s, "GET", "/pilot/room", host["token"].(string), nil)
		if seconds == 300 && room["state"] != "ended" {
			t.Fatal("invitation stayed open", room)
		}
	}
	if code, _ := request(s, "POST", "/pilot/join", "", map[string]string{"invite": host["invite"].(string), "name": "test"}); code != 401 {
		t.Fatal(code)
	}
}

func TestAuditIsBoundedPrivateAndRedacted(t *testing.T) {
	s, admin, host := fixture(t)
	_, viewer := request(s, "POST", "/pilot/join", "", map[string]string{"invite": host["invite"].(string), "name": "PRIVATE_VIEWER_NAME"})
	for _, token := range []string{"", host["token"].(string), viewer["token"].(string)} {
		if code, _ := request(s, "GET", "/pilot/security", token, nil); code != 401 {
			t.Fatal("audit leaked", code)
		}
	}
	for i := 0; i < 200; i++ {
		s.record(nil, "authentication_rejected")
	}
	code, audit := request(s, "GET", "/pilot/security", admin, nil)
	if code != 200 || len(audit["events"].([]any)) != 128 {
		t.Fatal(code, audit)
	}
	data, _ := json.Marshal(audit)
	for _, secret := range []string{admin, host["token"].(string), host["invite"].(string), viewer["token"].(string), "PRIVATE_VIEWER_NAME"} {
		if bytes.Contains(data, []byte(secret)) {
			t.Fatal("audit contains private data")
		}
	}
}

func TestBearerHeaderRequiredAndOversizeRejected(t *testing.T) {
	s, _, host := fixture(t)
	r := httptest.NewRequest("GET", "/pilot/room", nil)
	r.Header.Set("Authorization", host["token"].(string))
	w := httptest.NewRecorder()
	s.Handler().ServeHTTP(w, r)
	if w.Code != 401 {
		t.Fatal("bare token accepted", w.Code)
	}
	r = httptest.NewRequest("POST", "/pilot/join", strings.NewReader(strings.Repeat("x", maxBody+1)))
	w = httptest.NewRecorder()
	s.Handler().ServeHTTP(w, r)
	if w.Code != 413 {
		t.Fatal("oversized request accepted", w.Code)
	}
}

type blockedReader struct{ entered, release chan struct{} }

func (b *blockedReader) Read([]byte) (int, error) { close(b.entered); <-b.release; return 0, io.EOF }
func TestSlowUploadDoesNotBlockStop(t *testing.T) {
	s, _, host := fixture(t)
	body := &blockedReader{make(chan struct{}), make(chan struct{})}
	defer close(body.release)
	go s.Handler().ServeHTTP(httptest.NewRecorder(), httptest.NewRequest("POST", "/pilot/join", body))
	<-body.entered
	done := make(chan int, 1)
	go func() {
		code, _ := request(s, "POST", "/pilot/action", host["token"].(string), map[string]string{"action": "end"})
		done <- code
	}()
	select {
	case code := <-done:
		if code != 200 {
			t.Fatal(code)
		}
	case <-time.After(time.Second):
		t.Fatal("slow body held the room lock")
	}
}

type blockedWriter struct {
	http.ResponseWriter
	entered, release chan struct{}
}

func (b *blockedWriter) Write(p []byte) (int, error) {
	close(b.entered)
	<-b.release
	return len(p), nil
}
func TestSlowDownloadDoesNotBlockStop(t *testing.T) {
	s, _, host := fixture(t)
	w := &blockedWriter{httptest.NewRecorder(), make(chan struct{}), make(chan struct{})}
	defer close(w.release)
	r := httptest.NewRequest("GET", "/pilot/room", nil)
	r.Header.Set("Authorization", "Bearer "+host["token"].(string))
	go s.Handler().ServeHTTP(w, r)
	<-w.entered
	done := make(chan int, 1)
	go func() {
		code, _ := request(s, "POST", "/pilot/action", host["token"].(string), map[string]string{"action": "end"})
		done <- code
	}()
	select {
	case code := <-done:
		if code != 200 {
			t.Fatal(code)
		}
	case <-time.After(time.Second):
		t.Fatal("slow response held the room lock")
	}
}
