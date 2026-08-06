// Class-based counter with increment / reset.

#include <cstdio>

class Counter {
public:
    explicit Counter(int start = 0) : value_(start) {}

    int increment(int delta = 1) {
        value_ += delta;
        return value_;
    }

    void reset() { value_ = 0; }
    int value() const { return value_; }

private:
    int value_;
};

int main() {
    Counter c(10);
    std::printf("%d\n", c.increment());
    std::printf("%d\n", c.increment(5));
    c.reset();
    std::printf("%d\n", c.increment(3));
    return 0;
}