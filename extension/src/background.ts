import xhr from "sw-xhr";
globalThis.XMLHttpRequest = xhr as any;
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { LuteClient } from "./proto/LuteServiceClientPb";

export const core = new LuteClient("http://localhost:22000");

const main = async () => {
  const res = await core.healthCheck(new Empty(), null);
  console.log(`Health check: ${res.getOk() ? "OK" : "NOT OK"}`);
};

main();
