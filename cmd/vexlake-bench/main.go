// Package main implements the VexLake benchmark tool.
package main

import (
	"flag"
	"fmt"
	"log"
	"math/rand"
	"net"
	"sort"
	"sync"
	"sync/atomic"
	"time"
)

var (
	host        = flag.String("host", "localhost", "Server host")
	port        = flag.String("port", "6379", "Server port")
	concurrency = flag.Int("concurrency", 50, "Number of concurrent connections")
	numOps      = flag.Int("n", 100000, "Total number of operations")
	mode        = flag.String("mode", "insert", "Benchmark mode: insert or search")
	dimension   = flag.Int("dim", 128, "Vector dimension")
)

func main() {
	flag.Parse()

	addr := net.JoinHostPort(*host, *port)

	fmt.Println("=== VexLake Benchmark ===")
	fmt.Printf("Mode:        %s\n", *mode)
	fmt.Printf("Concurrency: %d\n", *concurrency)
	fmt.Printf("Total Ops:   %d\n", *numOps)
	fmt.Printf("Dimension:   %d\n", *dimension)
	fmt.Println("---")

	var success, errors int64
	var latencies []time.Duration
	var mu sync.Mutex
	var wg sync.WaitGroup

	opsPerWorker := *numOps / *concurrency
	start := time.Now()

	for i := 0; i < *concurrency; i++ {
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()

			conn, err := net.Dial("tcp", addr)
			if err != nil {
				log.Printf("Worker %d: connection failed: %v", workerID, err)
				atomic.AddInt64(&errors, int64(opsPerWorker))
				return
			}
			defer conn.Close()

			localLatencies := make([]time.Duration, 0, opsPerWorker)

			for j := 0; j < opsPerWorker; j++ {
				opStart := time.Now()
				var err error

				switch *mode {
				case "insert":
					err = doInsert(conn, workerID, j, *dimension)
				case "search":
					err = doSearch(conn, *dimension)
				default:
					err = fmt.Errorf("unknown mode: %s", *mode)
				}

				latency := time.Since(opStart)
				localLatencies = append(localLatencies, latency)

				if err != nil {
					atomic.AddInt64(&errors, 1)
				} else {
					atomic.AddInt64(&success, 1)
				}
			}

			mu.Lock()
			latencies = append(latencies, localLatencies...)
			mu.Unlock()
		}(i)
	}

	wg.Wait()
	elapsed := time.Since(start)

	// Calculate statistics
	sort.Slice(latencies, func(i, j int) bool {
		return latencies[i] < latencies[j]
	})

	var totalLatency time.Duration
	for _, l := range latencies {
		totalLatency += l
	}

	fmt.Printf("Total Time:  %v\n", elapsed)
	fmt.Printf("QPS:         %.0f ops/sec\n", float64(success)/elapsed.Seconds())
	fmt.Printf("Success:     %d\n", success)
	fmt.Printf("Errors:      %d\n", errors)
	fmt.Println()

	if len(latencies) > 0 {
		fmt.Println("Latency Statistics:")
		fmt.Printf("  Min:       %v\n", latencies[0])
		fmt.Printf("  Avg:       %v\n", totalLatency/time.Duration(len(latencies)))
		fmt.Printf("  P50:       %v\n", latencies[len(latencies)*50/100])
		fmt.Printf("  P95:       %v\n", latencies[len(latencies)*95/100])
		fmt.Printf("  P99:       %v\n", latencies[len(latencies)*99/100])
		fmt.Printf("  Max:       %v\n", latencies[len(latencies)-1])
	}
}

func doInsert(conn net.Conn, workerID, opID, dim int) error {
	key := fmt.Sprintf("vec:%d:%d", workerID, opID)
	vector := randomVector(dim)

	cmd := fmt.Sprintf("*3\r\n$4\r\nVSET\r\n$%d\r\n%s\r\n$%d\r\n%s\r\n",
		len(key), key, len(vector), vector)

	_, err := conn.Write([]byte(cmd))
	if err != nil {
		return err
	}

	buf := make([]byte, 32)
	_, err = conn.Read(buf)
	return err
}

func doSearch(conn net.Conn, dim int) error {
	vector := randomVector(dim)
	k := "10"

	cmd := fmt.Sprintf("*3\r\n$7\r\nVSEARCH\r\n$%d\r\n%s\r\n$%d\r\n%s\r\n",
		len(vector), vector, len(k), k)

	_, err := conn.Write([]byte(cmd))
	if err != nil {
		return err
	}

	buf := make([]byte, 4096)
	_, err = conn.Read(buf)
	return err
}

func randomVector(dim int) string {
	values := make([]string, dim)
	for i := 0; i < dim; i++ {
		values[i] = fmt.Sprintf("%.6f", rand.Float64())
	}

	result := "["
	for i, v := range values {
		if i > 0 {
			result += ", "
		}
		result += v
	}
	result += "]"
	return result
}
