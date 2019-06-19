# Table of contents
- Chapter 1 - [Language](https://github.com/calebwin/emu/edit/master/book/language.md#table-of-contents)
- Chapter 2 - [Holes](https://github.com/calebwin/emu/edit/master/book/holes.md#table-of-contents)
- Chapter 3 - [Numbers](https://github.com/calebwin/emu/edit/master/book/numbers.md#table-of-contents)

# Language
The Emu language comes with a bunch of constructs that should be familiar to you if you have programmed in a decently popular language before.
- If/Else statements
```rust
let is_midpoint: bool = true;
let position: i64 = 389;

if position % 2 == 0 || (is_midpoint && position % 2 == 1) {
	position += 10;
	is_midpoint = false;
}
```
- For loops
```rust
// "scores" and "scores_size" is a parameter to this kernel
// average of scores is computed

let total: f32 = 0
for i in 0..(scores_size - 1) {
	total += scores[i];
}
let average: f32 = total / scores_size;
```
- While loops
```rust
let age_limit: i8 = 120;
let age_max: i8 = 60;
let age_reached = false;

while age_limit > 20 {
	age_reached = age_max < age_limit
	age_limit -= 10;
}
```
- Infinite loops
- Break/Continue statements
```rust
let x: f32 = 0;
let dx: f32 = 1;
let x_max: f32 = 1 << 8;

loop {
	x += dx;
	if x > x_max { break; }
	if x > x_max/2 { continue; }
	x += dx/10;
}
```
- Return statements
```rust
return 0;
```
- Assignment statements - `=`, `+=`, `-=`, `*=`, `/=`, `%= `, `&=`, `^= `, `<<=`, `>>=`
- Index operator `[]`
- Call operator `()`
- Unary operators - `*` for dereferencing, `!` for negating booleans, `-` for negating numbers
- Binary operators - `+`, `-`, `*`, `/`, `%`, `&&`, `||`, `&`, `|`, `^`, `>>`, `<<`, `>`, `<`, `>=`, `<=`, `==`, `!=`
