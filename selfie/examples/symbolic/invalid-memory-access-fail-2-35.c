uint64_t main() {
  uint64_t  a;
  uint64_t* x;

  x = malloc(sizeof(uint64_t));

  *x = 0; // touch memory

  read(0, x, 1);

  if (*x == 48)
    // address outside of virtual address space -> invalid memory access
    // if the input is '0' (== 48 == b00110000)
    *(x + 4294967296) = 0;

  a = *x - 7;

  if (a == 42)
    // non-zero exit code if the input is '1' (== 49 == b00110001)
    return 1;
  else
    return 0;
}
