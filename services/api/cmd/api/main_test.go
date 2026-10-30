package main

import (
	"encoding/base64"
	"encoding/json"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func TestReadinessReflectsConfiguration(t *testing.T) {
	for _, enabled := range []bool{false, true} {
		t.Setenv("DX_PILOT_TOKEN_FILE", "")
		t.Setenv("DX_TURN_URL", "")
		t.Setenv("DX_TURN_SECRET_FILE", "")
		t.Setenv("DX_RELAY_ONLY", "")
		t.Setenv("DX_PILOT_GUEST_HOSTS", "1")
		if enabled {
			file := filepath.Join(t.TempDir(), "token")
			if err := os.WriteFile(file, []byte(base64.RawURLEncoding.EncodeToString(make([]byte, 32))), 0600); err != nil {
				t.Fatal(err)
			}
			t.Setenv("DX_PILOT_TOKEN_FILE", file)
		}
		h, err := handler()
		if err != nil {
			t.Fatal(err)
		}
		w := httptest.NewRecorder()
		h.ServeHTTP(w, httptest.NewRequest("GET", "/readyz", nil))
		var r map[string]any
		json.Unmarshal(w.Body.Bytes(), &r)
		if r["ready"] != enabled || (enabled && w.Code != 200) || (!enabled && w.Code != 503) {
			t.Fatal(w.Code, r)
		}
		w = httptest.NewRecorder()
		h.ServeHTTP(w, httptest.NewRequest("POST", "/v1/sessions", nil))
		if w.Code != 503 {
			t.Fatal("enterprise routes must remain unavailable")
		}
	}
}
