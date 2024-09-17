# Python bindings

## Running the example

To build the module run `./build.sh`, which will put a native module called `ctpv_rs_python_bindings.so` into
the `./examples` folder.
Then in a python virtual environment with `numpy` installed, you can run `python python-cptv-decoder.py` from inside
the `examples` folder.

## Release to pypi

Update `pyproject.toml` version identifier then create a release on github this will upload a new version to https://pypi.org/project/python-cptv
