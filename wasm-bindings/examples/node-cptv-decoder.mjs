import {CptvDecoderContext} from "../pkg/wasm_bindings.js";
import fs from "fs";

const FakeReader = function (bytes, maxChunkSize = 0) {
    let state = {
        offsets: [],
        offset: 0
    };
    state.bytes = bytes;
    const length = bytes.byteLength;
    // How many reader chunks to split the file into
    let numParts = 5;
    if (maxChunkSize !== 0) {
        numParts = Math.ceil(length / maxChunkSize);
    }
    const percentages = length / numParts;
    for (let i = 0; i < numParts; i++) {
        state.offsets.push(Math.ceil(percentages * i));
    }
    state.offsets.push(length);
    return {
        read() {
            return new Promise((resolve) => {
                state.offset += 1;
                const value = state.bytes.slice(state.offsets[state.offset - 1], state.offsets[state.offset]);
                resolve({
                    value,
                    done: state.offset === state.offsets.length - 1
                });
            });
        },
        cancel() {
            // Reset state
            delete state.bytes;
            state = {
                offsets: [],
                offset: 0
            };
            return new Promise((resolve) => {
                resolve()
            });
        }
    }
};

(async function main() {
    const buffer = fs.readFileSync("../../cptv-codec-rs/tests/fixtures/748923-20201221.cptv");
    const reader = new FakeReader(buffer, 100000);
    const start = performance.now();
    // TODO: Handle stream cancellation
    const decoderContext = CptvDecoderContext.newWithReadableStream(reader);
    const _header = await decoderContext.getHeader();
    let frame;
    let num = 0;
    while (frame = await decoderContext.nextFrameOwned() && frame !== null) {
        num++;
    }
    console.log(performance.now() - start);
    console.log(num);
    // TODO: Should header be filled with minValue, maxValue, totalFrames if it doesn't have those fields?
}());