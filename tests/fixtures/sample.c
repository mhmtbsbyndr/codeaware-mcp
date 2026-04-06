#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int x;
    int y;
} Point;

struct Config {
    char* name;
    int value;
};

enum Color {
    RED,
    GREEN,
    BLUE
};

typedef int (*Callback)(int, int);

void greet(const char* name) {
    printf("Hello, %s\n", name);
}

int add(int a, int b) {
    return a + b;
}

static int helper(void) {
    return 42;
}
