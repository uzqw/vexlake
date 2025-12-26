package core

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lvexlake_core
#include <stdlib.h>

int vexlake_health_check();
const char* vexlake_version();
int vexlake_init(int dim);
void vexlake_shutdown();
int vexlake_insert(unsigned long long id, const float* vec_ptr, int len);
char* vexlake_search(const float* query_ptr, int len, int k, int ef);
void vexlake_free_string(char* ptr);
*/
import "C"

import (
	"encoding/json"
	"fmt"
)

// SearchResult matches the Rust SearchResult struct
type SearchResult struct {
	ID    uint64  `json:"id"`
	Score float32 `json:"score"`
}

// Init initializes the Rust engine
func Init(dim int) error {
	res := C.vexlake_init(C.int(dim))
	if res != 0 {
		return fmt.Errorf("failed to initialize Rust engine (code: %d)", res)
	}
	return nil
}

// Shutdown cleans up the Rust engine
func Shutdown() {
	C.vexlake_shutdown()
}

// HealthCheck checks if the engine is functional
func HealthCheck() bool {
	return C.vexlake_health_check() == 1
}

// Version returns the engine version
func Version() string {
	return C.GoString(C.vexlake_version())
}

// Insert adds a vector to the index
func Insert(id uint64, vec []float32) error {
	if len(vec) == 0 {
		return fmt.Errorf("empty vector")
	}
	res := C.vexlake_insert(C.ulonglong(id), (*C.float)(&vec[0]), C.int(len(vec)))
	if res != 0 {
		return fmt.Errorf("failed to insert vector (code: %d)", res)
	}
	return nil
}

// Search find the nearest neighbors for a query vector
func Search(query []float32, k, ef int) ([]SearchResult, error) {
	if len(query) == 0 {
		return nil, fmt.Errorf("empty query")
	}

	ptr := C.vexlake_search((*C.float)(&query[0]), C.int(len(query)), C.int(k), C.int(ef))
	if ptr == nil {
		return nil, fmt.Errorf("search failed")
	}
	defer C.vexlake_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var results []SearchResult
	if err := json.Unmarshal([]byte(jsonStr), &results); err != nil {
		return nil, fmt.Errorf("failed to parse search results: %w", err)
	}

	return results, nil
}
