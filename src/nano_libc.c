#ifdef KernelPrinting
extern void seL4_DebugPutChar(char);
#endif

extern char * strcpy( char * s1, const char * s2 )
{
  char * rc = s1;
  while ( ( *s1++ = *s2++ ) );
  return rc;
}


void debug_puts(char* s) {
  #ifdef KernelPrinting
  while ( *s != '\0' ) {
    seL4_DebugPutChar(*s++);
  }
  #endif
}

extern void __assert_fail(char *expr, char* file, int line, char* func) {
  // Warning prevention
  (void) line;

  debug_puts("ASSERT ");
  debug_puts(expr);
  debug_puts(" in ");
  debug_puts(func);
  debug_puts(" at ");
  debug_puts(file);
  debug_puts(":??");

  __builtin_trap();
}
