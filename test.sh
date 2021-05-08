cargo build --release
cp target/release/libcleanax.so tests/
mv tests/libcleanax.so tests/cleanax.so
cd tests/
time python test_cleanax.py
cd ../