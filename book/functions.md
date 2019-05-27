# Functions
Emu supports a small set of functions that allow you to manage work and items that the GPU manages.
- `get_work_dim()` returns number of dimensions
- `get_global_size()` returns number of global work items specified for given dimension
- `get_global_id()` returns unique ID of global work-item for given dimension
- `get_local_size()` returns number of local work items specified for given dimension
- `get_local_id()` returns unique ID of local work-item that is within a specific work-group for given dimension
- `get_num_groups()` returns number of work-groups for given dimension
- `get_group_id()` returns ID of work-group
