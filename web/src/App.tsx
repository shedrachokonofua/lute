import React, { useEffect, useState } from "react";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { core } from "./client";

export const App = () => {
  const [ok, setOk] = useState<boolean | null>(null);
  useEffect(() => {
    core
      .healthCheck(new Empty(), null)
      .then((res) => {
        setOk(res.getOk());
      })
      .catch((err) => {
        console.error(err);
        setOk(false);
      });
  }, []);

  return (
    <div>Healthcheck: {ok === null ? "Loading..." : ok ? "OK" : "Not OK"}</div>
  );
};
