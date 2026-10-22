# Design: Size the document budget from the host's memory

`budget::available_memory()` reads the host once per `ImageBudget::default()`,
so each open sees what is free at that moment:

- Linux: `MemAvailable` from `/proc/meminfo`, the kernel's estimate of memory
  available for new work, which counts reclaimable cache.
- macOS: `host_statistics64(HOST_VM_INFO64)`, taking free, inactive, and
  purgeable pages times the page size.
- Windows: `GlobalMemoryStatusEx().ullAvailPhys`.
- Anything else: `None`.

`document_bytes(available, per_allocation)` is `available / 4 * 3`, floored at
one `per_allocation` plane so an ordinary document always opens. For `None`
it is eight planes, the previous fixed limit. The quarter held back covers
what an open document needs beyond its decoded planes: the composite, view
pyramid, history, GPU staging, and other processes.

The field is public, so a caller or test can pin it. The tests that measure a
document total now set it explicitly, so they do not depend on the CI host.

ponytail: available memory is a snapshot. Memory freed or taken after the
budget is read is not seen, and the macOS and Windows readers are verified
only by those platforms' CI builds.
