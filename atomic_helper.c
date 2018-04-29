#include <stdint.h>
#include <stdatomic.h>

void atomic_write16(const uint16_t *ptr, uint16_t val) {
    atomic_store((_Atomic uint16_t*)ptr, val);
}
