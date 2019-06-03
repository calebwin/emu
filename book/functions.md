# The Emu Book
- Chapter 0 - [Introduction](https://github.com/calebwin/emu/blob/master/book/introduction.md)
- Chapter 1 - [The Body](https://github.com/calebwin/emu/blob/master/book/body.md)
- Chapter 2 - [The Parameters](https://github.com/calebwin/emu/blob/master/book/parameters.md)
- Chapter 3 - [The Functions](https://github.com/calebwin/emu/blob/master/book/functions.md)
- Chapter 4 - [The Numbers](https://github.com/calebwin/emu/blob/master/book/numbers.md)
- Chapter 5 - [The Execution](https://github.com/calebwin/emu/blob/master/book/execution.md)

# Functions
Emu supports a small set of functions (lifted from OpenCL) that allow you to manage work and items that the GPU manages.
- `get_work_dim()` returns number of dimensions
- `get_global_size()` returns number of global work items specified for given dimension
- `get_global_id()` returns unique ID of global work-item for given dimension
- `get_local_size()` returns number of local work items specified for given dimension
- `get_local_id()` returns unique ID of local work-item that is within a specific work-group for given dimension
- `get_num_groups()` returns number of work-groups for given dimension
- `get_group_id()` returns ID of work-group

The most common function you will need is `get_global_id()`, especially if you rely on `build!` and the functions it generates to execute code. `get_global_id(0)` will give you the current index of the vector whose elements your function is executed on.
