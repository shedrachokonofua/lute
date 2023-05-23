import xhr from "sw-xhr";
globalThis.XMLHttpRequest = xhr as any;
