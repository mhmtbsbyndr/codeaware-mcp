#include <string>
#include <vector>

namespace utils {

class Logger {
public:
    Logger(const std::string& name) : name_(name) {}

    void log(const std::string& message) {
        // log implementation
    }

    int getLevel() const {
        return level_;
    }

private:
    std::string name_;
    int level_ = 0;
};

struct Point {
    double x;
    double y;
};

enum class Color {
    Red,
    Green,
    Blue
};

} // namespace utils

template<typename T>
class Container {
public:
    void add(const T& item) {
        items_.push_back(item);
    }

    int size() const {
        return items_.size();
    }

private:
    std::vector<T> items_;
};

void freeFunction(int x) {
    // free function
}

typedef int IntAlias;
