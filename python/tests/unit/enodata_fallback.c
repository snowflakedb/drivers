/* errno.h's guard is set, so this file's include does not define ENODATA. */
#include <errno.h>
#undef ENODATA
#include "nanoarrow_ipc.c"

int main(void) { return ENODATA == 120 ? 0 : 1; }
