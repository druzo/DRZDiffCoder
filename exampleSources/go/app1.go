// Go — struct + json marshal + simple persistence.

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"
)

type Task struct {
	Title    string    `json:"title"`
	Priority int       `json:"priority"`
	Due      time.Time `json:"due"`
}

func sortByPriority(in []Task) []Task {
	out := make([]Task, len(in))
	copy(out, in)
	for i := 1; i < len(out); i++ {
		for j := i; j > 0 && out[j-1].Priority > out[j].Priority; j-- {
			out[j-1], out[j] = out[j], out[j-1]
		}
	}
	return out
}

func main() {
	backlog := []Task{
		{Title: "Write tests", Priority: 2, Due: time.Now().Add(48 * time.Hour)},
		{Title: "Fix login bug", Priority: 5, Due: time.Now().Add(24 * time.Hour)},
		{Title: "Refactor parser", Priority: 3, Due: time.Now().Add(72 * time.Hour)},
	}

	ordered := sortByPriority(backlog)
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	for _, t := range ordered {
		_ = enc.Encode(t)
		fmt.Println("---")
	}
}