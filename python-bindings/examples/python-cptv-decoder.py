import cptv_rs_python_bindings
import time

start = time.time()
cptv_reader = cptv_rs_python_bindings.CptvReader(
    "../../cptv-codec-rs/tests/fixtures/20201221-748923.cptv")
reader = cptv_reader.get_header()

print("version:", reader.version)
print("device name:", reader.device_name)
print("device id:", reader.device_id)
print("time:", reader.timestamp)
print("dims:", reader.x_resolution, reader.y_resolution)
print("location:", reader.latitude, reader.longitude)
print("location timestamp:", reader.loc_timestamp)
print("altitude:", reader.altitude)

print("preview secs:", reader.preview_secs)
print("motion config:", reader.loc_timestamp)

print("fps:", reader.fps)
print("model:", reader.model)
print("brand:", reader.brand)
print("firmware:", reader.firmware)
print("camera_serial:", reader.camera_serial)

t0 = None
frame_num = 0

while True:
    frame = cptv_reader.next_frame()
    if frame is not None:
        if t0 is None:
            t0 = frame.time_on
        frame_num += 1
        print(
            f"{frame.time_on - t0} ({frame.time_on}) - ffc: {frame.time_on - frame.last_ffc_time}, min: {frame.pix.min()}, max: {frame.pix.max()}, temp_c: {frame.temp_c}, last_ffc_temp_c: {frame.last_ffc_temp_c}"
        )
    else:
        break

end = time.time()
print(f"{frame_num}, elapsed {end - start}")
