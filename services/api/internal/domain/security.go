// Package domain contains the phase-0 control-plane security kernel.
// It is not a deployed authentication or persistence service.
package domain

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"sync"
	"time"
)

type Capability string

const (
	ViewScreen     Capability = "view_screen"
	ControlInput   Capability = "control_input"
	FileSend       Capability = "file_send"
	FileReceive    Capability = "file_receive"
	ClipboardRead  Capability = "clipboard_read"
	ClipboardWrite Capability = "clipboard_write"
)

var ErrDenied = errors.New("access_denied")
var ErrExpired = errors.New("expired_or_used")
var ErrConflict = errors.New("state_conflict")

type Principal struct {
	Issuer, Subject string
	MFA             bool
}
type Assignment struct {
	Issuer, Subject, Tenant, Device string
	Allowed                         map[Capability]bool
	Revoked                         bool
}

// Platform admin status deliberately does not bypass device assignment or MFA.
func Effective(p Principal, tenant, device string, a Assignment, scopes ...map[Capability]bool) (map[Capability]bool, error) {
	if !p.MFA || p.Issuer == "" || p.Subject == "" || a.Revoked || p.Issuer != a.Issuer || p.Subject != a.Subject || tenant != a.Tenant || device != a.Device {
		return nil, ErrDenied
	}
	// customer, device and session permissions must all be present.
	if len(scopes) != 3 {
		return nil, ErrDenied
	}
	result := make(map[Capability]bool)
	for c, yes := range a.Allowed {
		if !yes {
			continue
		}
		allowed := true
		for _, s := range scopes {
			if !s[c] {
				allowed = false
				break
			}
		}
		if allowed {
			result[c] = true
		}
	}
	return result, nil
}

type tokenRecord struct {
	tenant   string
	expires  time.Time
	consumed bool
}

// TokenVault is an in-memory test adapter only. Production uses the atomic
// PostgreSQL UPDATE ... RETURNING path and stores only SHA-256 of 256-bit tokens.
type TokenVault struct {
	mu      sync.Mutex
	records map[[32]byte]tokenRecord
}

func NewTokenVault() *TokenVault { return &TokenVault{records: make(map[[32]byte]tokenRecord)} }
func (v *TokenVault) Issue(tenant string, now time.Time, ttl time.Duration) (string, error) {
	if tenant == "" || ttl <= 0 || ttl > 24*time.Hour {
		return "", ErrDenied
	}
	bytes := make([]byte, 32)
	if _, e := rand.Read(bytes); e != nil {
		return "", e
	}
	token := base64.RawURLEncoding.EncodeToString(bytes)
	v.mu.Lock()
	defer v.mu.Unlock()
	for k, r := range v.records {
		if !now.Before(r.expires) {
			delete(v.records, k)
		}
	}
	if len(v.records) >= 10_000 {
		return "", ErrConflict
	}
	v.records[sha256.Sum256([]byte(token))] = tokenRecord{tenant: tenant, expires: now.Add(ttl)}
	return token, nil
}
func (v *TokenVault) Consume(token, tenant string, now time.Time) error {
	if len(token) != 43 {
		return ErrExpired
	}
	hash := sha256.Sum256([]byte(token))
	v.mu.Lock()
	defer v.mu.Unlock()
	r, ok := v.records[hash]
	if !ok || r.tenant != tenant || r.consumed || !now.Before(r.expires) {
		return ErrExpired
	}
	r.consumed = true
	v.records[hash] = r
	return nil
}

type State string

const (
	Requested       State = "requested"
	AwaitingConsent State = "awaiting_consent"
	Negotiating     State = "negotiating"
	Authenticating  State = "authenticating"
	Active          State = "active"
	Reconnecting    State = "reconnecting"
	Ended           State = "ended"
	Failed          State = "failed"
)

func CanTransition(from, to State) bool {
	if from == Ended || from == Failed {
		return false
	}
	switch from {
	case Requested, AwaitingConsent, Negotiating, Authenticating, Active, Reconnecting:
	default:
		return false
	}
	if to == Ended || to == Failed {
		return true
	}
	return (from == Requested && to == AwaitingConsent) || (from == AwaitingConsent && to == Negotiating) || (from == Negotiating && to == Authenticating) || (from == Authenticating && to == Active) || (from == Active && to == Reconnecting) || (from == Reconnecting && to == Authenticating)
}

// Session serializes consent/cancel and tracks finite deadlines. Durable storage
// must use row locking/CAS, not instantiate independent copies of this adapter.
type Session struct {
	mu                         sync.Mutex
	State                      State
	Tenant, Device, Technician string
	Deadline                   time.Time
	PolicyVersion              uint64
	consumed                   bool
}

func (s *Session) Consent(tenant, device string, accepted bool, now time.Time) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if tenant != s.Tenant || device != s.Device {
		return ErrDenied
	}
	if s.State != AwaitingConsent || s.consumed {
		return ErrConflict
	}
	if !now.Before(s.Deadline) {
		s.State = Failed
		return ErrExpired
	}
	s.consumed = true
	if accepted {
		s.State = Negotiating
		s.Deadline = now.Add(30 * time.Second)
	} else {
		s.State = Ended
	}
	return nil
}
func (s *Session) Revoke() { s.mu.Lock(); defer s.mu.Unlock(); s.State = Ended; s.consumed = true }
func (s *Session) Renew(tenant, device, user string, policy uint64, now time.Time, seconds int) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.State != Active || tenant != s.Tenant || device != s.Device || user != s.Technician || policy != s.PolicyVersion {
		return ErrDenied
	}
	if !now.Before(s.Deadline) {
		s.State = Ended
		return ErrExpired
	}
	if seconds <= 0 || seconds > 60 {
		return ErrDenied
	}
	s.Deadline = now.Add(time.Duration(seconds) * time.Second)
	return nil
}
