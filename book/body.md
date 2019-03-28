# Body
The body of a kernel function consists of a series of statements. The general structure of statements can be explained with the following categorization of code in the Emu language.
- If/Else statements
- For loops
- While loops
- Infinite loops
- Break/Continue statements
- Assignment statements - `=`, `+=`, `-=`, `*=`, `/=`, `%= `, `&=`, `^= `, `<<=`, `>>=`
- Index operator `[]`
- Call operator `()`
- Unary operators - `*` for dereferencing, `!` for negating booleans, `-` for negating numbers
- Binary operators - `+`, `-`, `*`, `/`, `%`, `&&`, `||`, `&`, `|`, `^`, `>>`, `<<`, `>`, `<`, `>=`, `<=`, `==`, `!=`

The following is a list of features that are yet to be introduced to the Emu language.
[ ] Precision conversion (type casting)
[ ] SI units conversion
[x] Support for multiple kernels defined within the body of a single `emu` call
[ ] Support for constants, vectors, images
[ ] Support for better error reports