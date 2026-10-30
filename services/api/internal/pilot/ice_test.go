package pilot

import (
	"crypto/hmac"
	"crypto/sha1"
	"encoding/base64"
	"strings"
	"testing"
)

func TestTURNRequiresConsentAndUsesScopedExpiringCredentials(t *testing.T) {
	s, _, created := fixture(t)
	secret := strings.Repeat("s", 48)
	if err := s.ConfigureTURN("turn:relay.example.com:3478?transport=udp", secret, true); err != nil {
		t.Fatal(err)
	}
	host := created["token"].(string)
	_, joined := request(s, "POST", "/pilot/join", "", map[string]string{"invite": created["invite"].(string), "name": "viewer"})
	viewer := joined["token"].(string)
	for _, token := range []string{host, viewer} {
		_, r := request(s, "GET", "/pilot/room", token, nil)
		if r["ice"] != nil {
			t.Fatal("credentials disclosed before consent")
		}
	}
	_, accepted := request(s, "POST", "/pilot/action", host, map[string]string{"action": "accept"})
	config := accepted["ice"].(map[string]any)
	if config["iceTransportPolicy"] != "relay" {
		t.Fatal(config)
	}
	turn := config["iceServers"].([]any)[1].(map[string]any)
	username := turn["username"].(string)
	if !strings.HasSuffix(username, ":"+accepted["id"].(string)+":host") {
		t.Fatal("unscoped credential")
	}
	mac := hmac.New(sha1.New, []byte(secret))
	mac.Write([]byte(username))
	if turn["credential"] != base64.StdEncoding.EncodeToString(mac.Sum(nil)) {
		t.Fatal("coturn HMAC mismatch")
	}
	_, vr := request(s, "GET", "/pilot/room", viewer, nil)
	vt := vr["ice"].(map[string]any)["iceServers"].([]any)[1].(map[string]any)
	if vt["username"] == username {
		t.Fatal("roles must have distinct TURN quotas")
	}
	request(s, "POST", "/pilot/action", host, map[string]string{"action": "end"})
	_, ended := request(s, "GET", "/pilot/room", viewer, nil)
	if ended["ice"] != nil {
		t.Fatal("ended room issued credentials")
	}
}
func TestTURNConfigurationFailsClosed(t *testing.T) {
	for _, address := range []string{"turns:relay.example:5349", "turn:relay.example:3478?transport=tcp", "turn:relay.example", "turn://user:pass@relay.example:3478", "turn:relay.example:0?transport=udp", "turn:relay.example:99999?transport=udp"} {
		s, _, _ := fixture(t)
		if s.ConfigureTURN(address, strings.Repeat("x", 32), true) == nil {
			t.Fatalf("accepted %s", address)
		}
	}
	s, _, _ := fixture(t)
	if s.ConfigureTURN("", "", true) == nil {
		t.Fatal("relay allowed without relay server")
	}
	if s.ConfigureTURN("turn:relay.example:3478?transport=udp", "short", false) == nil {
		t.Fatal("weak secret")
	}
}
