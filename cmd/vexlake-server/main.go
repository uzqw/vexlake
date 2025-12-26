// Package main implements the VexLake server.
// This is the Go layer of the "Sandwich Architecture" - handling only:
// - RESP protocol parsing (via redcon)
// - Client connection management
// - Result formatting (Arrow → RESP)
//
// All heavy computation is delegated to the Rust core via CGO.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"

	"github.com/uzqw/vexlake/internal/core"

	"github.com/tidwall/redcon"
)

var (
	host      = flag.String("host", "0.0.0.0", "Host to bind to")
	port      = flag.String("port", "6379", "Port to listen on")
	dimension = flag.Int("dim", 128, "Vector dimension")
	version   = "dev"
)

func main() {
	flag.Parse()

	// Initialize Rust engine
	if err := core.Init(*dimension); err != nil {
		log.Fatalf("Failed to initialize VexLake core: %v", err)
	}
	defer core.Shutdown()

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

	log.Printf("VexLake server v%s starting on %s (dim=%d)", version, addr, *dimension)
	if err := server.ListenAndServe(); err != nil {
		log.Fatal(err)
	}
}

func handleCommand(conn redcon.Conn, cmd redcon.Command) {
	switch strings.ToUpper(string(cmd.Args[0])) {
	case "PING":
		handlePing(conn, cmd)
	case "ECHO":
		handleEcho(conn, cmd)
	case "QUIT":
		conn.WriteString("OK")
		conn.Close()
	case "INFO", "STATS":
		handleStats(conn)
	case "VSET":
		handleVSet(conn, cmd)
	case "VSEARCH":
		handleVSearch(conn, cmd)
	case "CLEAR":
		handleClear(conn)
	default:
		conn.WriteError("ERR unknown command '" + string(cmd.Args[0]) + "'")
	}
}

func handleAccept(conn redcon.Conn) bool {
	return true
}

func handleClose(conn redcon.Conn, err error) {
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
	stats := map[string]interface{}{
		"version": version,
		"status":  "ok",
		"engine":  "hnsw",
		"health":  core.HealthCheck(),
		"core_v":  core.Version(),
	}
	b, _ := json.Marshal(stats)
	conn.WriteBulk(b)
}

func handleVSet(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 3 {
		conn.WriteError("ERR wrong number of arguments for 'vset' command")
		return
	}

	id, err := strconv.ParseUint(string(cmd.Args[1]), 10, 64)
	if err != nil {
		conn.WriteError("ERR invalid id: must be uint64")
		return
	}

	vec, err := parseVector(string(cmd.Args[2]))
	if err != nil {
		conn.WriteError("ERR invalid vector: " + err.Error())
		return
	}

	if err := core.Insert(id, vec); err != nil {
		conn.WriteError("ERR insert failed: " + err.Error())
		return
	}

	conn.WriteString("OK")
}

func handleVSearch(conn redcon.Conn, cmd redcon.Command) {
	if len(cmd.Args) < 3 {
		conn.WriteError("ERR wrong number of arguments for 'vsearch' command")
		return
	}

	query, err := parseVector(string(cmd.Args[1]))
	if err != nil {
		conn.WriteError("ERR invalid query vector: " + err.Error())
		return
	}

	k, err := strconv.Atoi(string(cmd.Args[2]))
	if err != nil {
		conn.WriteError("ERR invalid k")
		return
	}

	ef := 50 // default ef
	if len(cmd.Args) > 3 {
		ef, _ = strconv.Atoi(string(cmd.Args[3]))
	}

	results, err := core.Search(query, k, ef)
	if err != nil {
		conn.WriteError("ERR search failed: " + err.Error())
		return
	}

	conn.WriteArray(len(results))
	for _, res := range results {
		conn.WriteBulkString(fmt.Sprintf("%d:%.4f", res.ID, res.Score))
	}
}

func handleClear(conn redcon.Conn) {
	core.Shutdown()
	core.Init(*dimension)
	conn.WriteString("OK")
}

func parseVector(s string) ([]float32, error) {
	s = strings.Trim(s, "[] ")
	parts := strings.Split(s, ",")
	vec := make([]float32, 0, len(parts))
	for _, p := range parts {
		p = strings.TrimSpace(p)
		if p == "" {
			continue
		}
		f, err := strconv.ParseFloat(p, 32)
		if err != nil {
			return nil, err
		}
		vec = append(vec, float32(f))
	}
	return vec, nil
}
