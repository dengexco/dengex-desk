// Phase-0 API process. Product routes stay unavailable until OIDC, PostgreSQL
// and device identity binding are integrated; there is no development auth bypass.
package main

import (
	"context"
	"dengex.local/remote/api/internal/pilot"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--healthcheck" {
		client := http.Client{Timeout: 3 * time.Second}
		response, err := client.Get("http://127.0.0.1:8080/healthz")
		if err != nil {
			os.Exit(1)
		}
		_ = response.Body.Close()
		if response.StatusCode != http.StatusOK {
			os.Exit(1)
		}
		return
	}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /{$}", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.Header().Set("Content-Security-Policy", "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		_, _ = w.Write([]byte(`<!doctype html><html lang="tr"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>dengeX Remote — Pilot</title><style>body{font:16px system-ui;background:#f4f8f7;color:#183d36;max-width:650px;margin:10vh auto;padding:30px}h1{font-size:36px}p{line-height:1.8}a{display:inline-block;background:#087f73;color:white;padding:14px 20px;border-radius:8px;text-decoration:none}</style><h1>dengeX Remote</h1><p>Mac veya Windows üzerinden karşı bilgisayarın ekranını izlemek için davetli geliştirme denemesi.</p><a href="/download/windows">Windows x64 paketini indir</a><p>ZIP’i açın, dengeX Remote.exe dosyasını çalıştırın. <b>Cihaza bağlan</b> bölümünde Windows cihazında davet oluşturun; kodu Mac uygulamasına girin. Windows kullanıcısı paylaşımı ayrıca kabul eder.</p><p>Geliştirme paketi Windows sertifikasıyla imzalanmamıştır. Windows cihazında çalıştırma doğrulaması bekleniyor. Bu pilot yalnızca görüntüleme destekler; uzaktan klavye/fare kontrolü içermez. WebView2 gerekir. Doğrudan WebRTC bağlantısı kurulamazsa ayrı TURN hizmeti gerekir.</p></html>`))
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
			log.Fatal("pilot token file unreadable")
		}
		server, err := pilot.New(strings.TrimSpace(string(token)))
		if err != nil {
			log.Fatal(err)
		}
		server.AllowGuestHosts = os.Getenv("DX_PILOT_GUEST_HOSTS") == "1"
		mux.Handle("/pilot/", server.Handler())
	}
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{"status": "ok", "phase": 0, "remoteSupportReady": false})
	})
	mux.HandleFunc("GET /readyz", func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "control_plane_not_integrated", http.StatusServiceUnavailable)
	})
	mux.HandleFunc("/v1/", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Cache-Control", "no-store")
		w.WriteHeader(http.StatusServiceUnavailable)
		_ = json.NewEncoder(w).Encode(map[string]string{"error": "control_plane_not_integrated"})
	})
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
	log.Printf("phase-0 health service listening on %s; product API unavailable", addr)
	if e := s.ListenAndServe(); e != nil && e != http.ErrServerClosed {
		log.Fatal(e)
	}
}
