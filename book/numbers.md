# Numbers
Numbers can be converted to be in terms of different units. Here's an example...
```rust
let distance: f32 = 19.2;

distance += 49.2 as cm;
distance += 2.9 as mm;
```
The above code will initialize `distance` to be in terms of meters and the subsequent additions will convert the quantities `49.2` and `2.9` to meters before adding.

The following are all supported unit prefixes-
- `Y` - `10^24`
- `Z` - `10^21`
- `E` - `10^18`
- `P` - `10^15`
- `T` - `10^12`
- `G` - `10^9`
- `M` - `10^6`
- `k` - `10^3`
- `h` - `10^2`
- `D` - `10^1`
- `d` - `10^-1`
- `c` - `10^-2`
- `m` - `10^-3`
- `u` - `10^-6`
- `n` - `10^-9`
- `p` - `10^-12`
- `f` - `10^-15`
- `a` - `10^-18`
- `z` - `10^-21`
- `y` - `10^-24`

Using the form - `x as y` will convert `x` to be in terms of `y` using the prefix of `y` from the above list to do the conversion.
