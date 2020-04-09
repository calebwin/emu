# run this from the emu_core directory
cargo +nightly rustdoc -p emu_core --target-dir ../docs --features glsl-compile -- --disable-per-crate-search -Z unstable-options
rm ../docs/index.html
cp ../docs/doc/emu_core/index.html ../docs/index.html
sed -i 's/href="/href="doc\/emu_core\//g' ../docs/index.html
sed -i 's/href='\''/href='\''doc\/emu_core\//g' ../docs/index.html
sed -i 's/src="/src="doc\/emu_core\//g' ../docs/index.html
sed -i 's/src='\''/src='\''doc\/emu_core\//g' ../docs/index.html
sed -i 's/href="doc\/emu_core\/https/href="https/g' ../docs/index.html
sed -i 's/href='\''doc\/emu_core\/https/href='\''https/g' ../docs/index.html
sed -i 's/src="doc\/emu_core\/https/src="https/g' ../docs/index.html
sed -i 's/src='\''doc\/emu_core\/https/src='\''https/g' ../docs/index.html