// Templated stack with bounds-checked pop.

#include <cstdio>
#include <stdexcept>
#include <vector>

template <typename T>
class Stack {
public:
    void push(T value) { data_.push_back(std::move(value)); }

    T pop() {
        if (data_.empty()) {
            throw std::runtime_error("pop on empty stack");
        }
        T top = std::move(data_.back());
        data_.pop_back();
        return top;
    }

    std::size_t size() const { return data_.size(); }
    bool empty() const { return data_.empty(); }

private:
    std::vector<T> data_;
};

int main() {
    Stack<int> s;
    for (int i = 1; i <= 5; ++i) {
        s.push(i * 10);
    }
    while (!s.empty()) {
        std::printf("%d ", s.pop());
    }
    std::printf("\n");
    return 0;
}