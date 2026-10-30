// Attended screen-sharing API. Enterprise /v1 routes remain unavailable.
package main

import (
	"context"
	"dengex.local/remote/api/internal/pilot"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

func handler() (http.Handler, error) {
	mux := http.NewServeMux()
	var enabled bool
	var guestHosts bool
	var turnEnabled bool

	mux.HandleFunc("GET /{$}", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		_, _ = w.Write([]byte(`<!doctype html><html lang="tr"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>dengeX Remote</title><style>body{font:18px system-ui;max-width:700px;margin:10vh auto;padding:24px;line-height:1.7}a{color:#087f73}</style><h1>dengeX Remote</h1><p>Windows ve macOS için açık kaynak, kullanıcı onaylı ekran paylaşımı.</p><p><a href="https://github.com/dengexco/dengex-desk/releases">Masaüstü uygulamasını indir</a> · <a href="https://github.com/dengexco/dengex-desk/blob/main/docs/self-hosting.md">Sunucu kurulum kılavuzu</a></p><ol><li>İki bilgisayarda uygulamayı açın ve bu sitenin HTTPS adresini girin.</li><li>Ekranı paylaşacak bilgisayarda davet oluşturun.</li><li>İzleyen bilgisayarda daveti girin; paylaşan bilgisayarda isteği kabul edin.</li></ol><p>Bu sürüm yalnızca ekran izleme sunar. Klavye/fare kontrolü, dosya aktarımı ve katılımsız erişim içermez.</p></html>`))
	})
	mux.HandleFunc("GET /download/windows", func(w http.ResponseWriter, r *http.Request) {
		path := os.Getenv("DX_PILOT_WINDOWS_ZIP")
		if path == "" {
			http.Error(w, "Windows package unavailable", 503)
			return
		}
		if _, err := os.Stat(path); err != nil {
			http.Error(w, "Windows package is being built", 503)
			return
		}
		w.Header().Set("Content-Disposition", `attachment; filename="dengeX-Remote-Windows-x64.zip"`)
		w.Header().Set("X-Content-Type-Options", "nosniff")
		http.ServeFile(w, r, filepath.Clean(path))
	})
	if file := os.Getenv("DX_PILOT_TOKEN_FILE"); file != "" {
		token, err := os.ReadFile(file)
		if err != nil {
			return nil, fmt.Errorf("pilot token file unreadable")
		}
		server, err := pilot.New(strings.TrimSpace(string(token)))
		if err != nil {
			return nil, err
		}
		server.AllowGuestHosts = os.Getenv("DX_PILOT_GUEST_HOSTS") == "1"
		secret := ""
		if secretFile := os.Getenv("DX_TURN_SECRET_FILE"); secretFile != "" {
			data, err := os.ReadFile(secretFile)
			if err != nil {
				return nil, fmt.Errorf("TURN secret file unreadable")
			}
			secret = strings.TrimSpace(string(data))
		}
		if err := server.ConfigureTURN(os.Getenv("DX_TURN_URL"), secret, os.Getenv("DX_RELAY_ONLY") == "1"); err != nil {
			return nil, err
		}
		enabled, guestHosts, turnEnabled = true, server.AllowGuestHosts, os.Getenv("DX_TURN_URL") != ""
		mux.Handle("/pilot/", server.Handler())
	}
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{"status": "ok", "service": "dengex-remote", "version": "0.2.0-beta.1", "attendedSharingReady": enabled})
	})
	mux.HandleFunc("GET /readyz", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Cache-Control", "no-store")
		if !enabled {
			w.WriteHeader(http.StatusServiceUnavailable)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{"service": "dengex-remote", "protocol": 1, "ready": enabled, "guestHosts": guestHosts, "turn": turnEnabled, "capabilities": []string{"view_screen"}})
	})
	mux.HandleFunc("/v1/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Cache-Control", "no-store")
		w.WriteHeader(http.StatusServiceUnavailable)
		_ = json.NewEncoder(w).Encode(map[string]string{"error": "control_plane_not_integrated"})
	})
	return mux, nil
}

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--healthcheck" {
		client := http.Client{Timeout: 3 * time.Second}
		response, err := client.Get("http://127.0.0.1:8080/readyz")
		if err != nil {
			os.Exit(1)
		}
		_ = response.Body.Close()
		if response.StatusCode != http.StatusOK {
			os.Exit(1)
		}
		return
	}
	mux, err := handler()
	if err != nil {
		log.Fatal(err)
	}

	addr := os.Getenv("DX_LISTEN")
	if addr == "" {
		addr = "127.0.0.1:8080"
	}
	s := http.Server{Addr: addr, Handler: mux, ReadHeaderTimeout: 5 * time.Second, ReadTimeout: 10 * time.Second, WriteTimeout: 120 * time.Second, IdleTimeout: 30 * time.Second, MaxHeaderBytes: 16 << 10}
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	go func() {
		<-ctx.Done()
		shutdown, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		_ = s.Shutdown(shutdown)
	}()
	log.Printf("dengeX attended-sharing API listening on %s", addr)
	if e := s.ListenAndServe(); e != nil && e != http.ErrServerClosed {
		log.Fatal(e)
	}
}
