// Go — interface + error wrapping.

package main

import (
	"errors"
	"fmt"
)

type Priced interface {
	Total() float64
}

type LineItem struct {
	Name     string
	Price    float64
	Quantity int
}

func (l LineItem) Total() float64 {
	return l.Price * float64(l.Quantity)
}

type Order struct {
	Items []LineItem
}

func (o Order) Total() (float64, error) {
	if len(o.Items) == 0 {
		return 0, errors.New("empty order")
	}
	var sum float64
	for _, it := range o.Items {
		sum += it.Total()
	}
	return sum, nil
}

func printTotal(p Priced) {
	switch v := p.(type) {
	case Order:
		t, err := v.Total()
		if err != nil {
			fmt.Println("err:", err)
			return
		}
		fmt.Printf("order total = %.2f\n", t)
	default:
		fmt.Printf("priced: %T = %.2f\n", v, v.Total())
	}
}

func main() {
	o := Order{Items: []LineItem{
		{Name: "Book", Price: 9.99, Quantity: 2},
		{Name: "Mug", Price: 6.50, Quantity: 1},
	}}
	printTotal(o)
}