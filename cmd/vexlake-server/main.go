// Package main implements the VexLake server.
// This is the Go layer of the "Sandwich Architecture" - handling only:
// - RESP protocol parsing (via redcon)
// - Client connection management
// - Result formatting (Arrow → RESP)
//
// All heavy computation is delegated to the Rust core via CGO.
package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/tidwall/redcon"
)

var (
	host    = flag.String("host", "0.0.0.0", "Host to bind to")
	port    = flag.String("port", "6379", "Port to listen on")
	version = "dev"
)

func main() {
	flag.Parse()

	addr := fmt.Sprintf("%s:%s", *host, *port)

	// Create server
	server := redcon.NewServer(addr,
		handleCommand,
		handleAccept,
		handleClose,
	)

	// Handle graceful shutdown
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-sigCh
		log.Println("Shutting down...")
		server.Close()
	}()

	log.Printf("VexLake server v%s starting on %s", version, addr)
	if err := server.ListenAndServe(); err != nil {
		log.Fatal(err)
	}
}

func handleCommand(conn redcon.Conn, cmd redcon.Command) {
	switch string(cmd.Args[0]) {
	case "PING", "ping":
		handlePing(conn, cmd)
	case "ECHO", "echo":
		handleEcho(conn, cmd)
	case "QUIT", "quit":
		conn.WriteString("OK")
		conn.Close()
	case "INFO", "info", "STATS", "stats":
		handleStats(conn)
	case "VSET", "vset":
		handleVSet(conn, cmd)
	case "VGET", "vget":
		handleVGet(conn, cmd)
	case "VSEARCH", "vsearch":
		handleVSearch(conn, cmd)
	case "VDEL", "vdel":
		handleVDel(conn, cmd)
	case "CLEAR", "clear":
		handleClear(conn)
	default:
		conn.WriteError("ERR unknown command '" + string(cmd.Args[0]) + "'")
	}
}

func handleAccept(conn redcon.Conn) bool {
	log.Printf("Client connected: %s", conn.RemoteAddr())
	return true
}

func handleClose(conn redcon.Conn, err error) {
	log.Printf("Client disconnected: %s", conn.RemoteAddr())
}

func handlePing(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) > 1 {
		conn.WriteBulk(cmd.Args[1])
	} else {
		conn.WriteString("PONG")
	}
}

func handleEcho(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 2 {
		conn.WriteError("ERR wrong number of arguments for 'echo' command")
		return
	}
	conn.WriteBulk(cmd.Args[1])
}

func handleStats(conn redcon.Conn) {
	// TODO: Get stats from Rust core
	stats := `{"version":"` + version + `","status":"ok"}`
	conn.WriteBulk([]byte(stats))
}

func handleVSet(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 3 {
		conn.WriteError("ERR wrong number of arguments for 'vset' command")
		return
	}
	// TODO: Call Rust core via CGO
	// key := string(cmd.Args[1])
	// vector := string(cmd.Args[2])
	conn.WriteString("OK")
}

func handleVGet(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 2 {
		conn.WriteError("ERR wrong number of arguments for 'vget' command")
		return
	}
	// TODO: Call Rust core via CGO
	conn.WriteNull()
}

func handleVSearch(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 3 {
		conn.WriteError("ERR wrong number of arguments for 'vsearch' command")
		return
	}
	// TODO: Call Rust core via CGO
	conn.WriteArray(0)
}

func handleVDel(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 2 {
		conn.WriteError("ERR wrong number of arguments for 'vdel' command")
		return
	}
	// TODO: Call Rust core via CGO
	conn.WriteInt(0)
}

func handleClear(conn redcon.Conn) {
	// TODO: Call Rust core via CGO
	conn.WriteString("OK")
}
