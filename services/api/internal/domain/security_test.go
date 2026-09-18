package domain

import (
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

func TestTenantAndMFABoundary(t *testing.T) {
	p := Principal{"issuer", "tech", true}
	a := Assignment{Issuer: "issuer", Subject: "tech", Tenant: "a", Device: "d", Allowed: map[Capability]bool{ViewScreen: true, ControlInput: true}}
	view := map[Capability]bool{ViewScreen: true}
	for _, tenant := range []string{"b", ""} {
		if _, e := Effective(p, tenant, "d", a, view, view, view); e == nil {
			t.Fatal("cross tenant allowed")
		}
	}
	c, e := Effective(p, "a", "d", a, view, view, view)
	if e != nil || !c[ViewScreen] || c[ControlInput] {
		t.Fatal("intersection failed")
	}
	p.MFA = false
	if _, e = Effective(p, "a", "d", a, view, view, view); e == nil {
		t.Fatal("missing MFA allowed")
	}
}
func TestConcurrentEnrollmentTokenConsumption(t *testing.T) {
	v := NewTokenVault()
	now := time.Now()
	token, e := v.Issue("a", now, time.Minute)
	if e != nil {
		t.Fatal(e)
	}
	if v.Consume(token, "b", now) == nil {
		t.Fatal("tenant bypass")
	}
	var successes atomic.Int32
	var wg sync.WaitGroup
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if v.Consume(token, "a", now) == nil {
				successes.Add(1)
			}
		}()
	}
	wg.Wait()
	if successes.Load() != 1 {
		t.Fatalf("consumptions=%d", successes.Load())
	}
}
func TestExpiredToken(t *testing.T) {
	v := NewTokenVault()
	now := time.Now()
	token, _ := v.Issue("a", now, time.Second)
	if v.Consume(token, "a", now.Add(time.Second)) == nil {
		t.Fatal("expired token accepted")
	}
}
func TestConsentCancelRace(t *testing.T) {
	for i := 0; i < 100; i++ {
		s := Session{State: AwaitingConsent, Tenant: "a", Device: "d", Deadline: time.Now().Add(time.Minute)}
		var wg sync.WaitGroup
		wg.Add(2)
		go func() { defer wg.Done(); _ = s.Consent("a", "d", true, time.Now()) }()
		go func() { defer wg.Done(); s.Revoke() }()
		wg.Wait()
		if s.State != Ended {
			t.Fatal("cancel lost")
		}
		if s.Consent("a", "d", true, time.Now()) == nil {
			t.Fatal("reopened after cancel")
		}
	}
}
func TestLeaseCannotReviveOrExceed60Seconds(t *testing.T) {
	now := time.Now()
	s := Session{State: Active, Tenant: "a", Device: "d", Technician: "u", PolicyVersion: 1, Deadline: now.Add(time.Second)}
	if s.Renew("a", "d", "u", 1, now, 61) == nil {
		t.Fatal("oversized lease")
	}
	if s.Renew("a", "d", "u", 2, now, 60) == nil {
		t.Fatal("stale policy")
	}
	if s.Renew("a", "d", "u", 1, now.Add(time.Second), 60) == nil {
		t.Fatal("expired lease revived")
	}
}
func TestTransitions(t *testing.T) {
	if CanTransition(Requested, Active) || CanTransition(Ended, Active) || CanTransition(State("junk"), Failed) {
		t.Fatal("invalid transition")
	}
	if !CanTransition(Active, Reconnecting) {
		t.Fatal("valid transition denied")
	}
}
func FuzzStateMachine(f *testing.F) {
	f.Add("ended", "active")
	f.Fuzz(func(t *testing.T, a, b string) {
		if (State(a) == Ended || State(a) == Failed) && CanTransition(State(a), State(b)) {
			t.Fatal("terminal state reopened")
		}
	})
}
