uint64_t main() {
    uint64_t* x;

    x = malloc(sizeof(uint64_t));

    *x = 0;

    read(0, x, 1);
    *x = *x - 50;

    return 1 / *x; // div by zero if input is '2' == 50 ASCII
}
