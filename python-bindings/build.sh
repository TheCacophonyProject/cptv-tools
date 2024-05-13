cargo build --release
# Put the native lib in the examples folder.
mv ../target/release/libcptv_rs_python_bindings.so ./examples/cptv_rs_python_bindings.so