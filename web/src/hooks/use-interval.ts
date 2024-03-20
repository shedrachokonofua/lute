import { useCallback, useEffect, useRef, useState } from "react";

const useSecondsRemaining = (intervalSeconds: number | null) => {
  const [value, setValue] = useState<number | null>(intervalSeconds);
  const secondsRemainingRef = useRef(intervalSeconds);

  useEffect(() => {
    secondsRemainingRef.current = intervalSeconds;
  }, [intervalSeconds]);

  const tick = useCallback(() => {
    if (secondsRemainingRef.current === null) {
      return;
    }
    secondsRemainingRef.current -= 1;
    setValue(secondsRemainingRef.current);
  }, []);

  const reset = useCallback(() => {
    setValue(intervalSeconds);
    secondsRemainingRef.current = intervalSeconds;
  }, [intervalSeconds]);

  return {
    value,
    setValue,
    ref: secondsRemainingRef,
    tick,
    reset,
  };
};

export const useInterval = (
  callback: () => void,
  intervalSeconds: number | null,
) => {
  const {
    value: secondsRemaining,
    ref: secondsRemainingRef,
    tick,
    reset,
  } = useSecondsRemaining(intervalSeconds);
  const callbackRef = useRef(callback);
  const intervalIdRef = useRef<number | null>(null);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    if (intervalSeconds === null) {
      return;
    }
    const intervalId = setInterval(() => {
      tick();
      if (secondsRemainingRef.current === 0) {
        callbackRef.current();
        reset();
      }
    }, 1000);
    intervalIdRef.current = intervalId;
    return () => {
      window.clearInterval(intervalId);
    };
  }, [intervalSeconds, secondsRemainingRef, tick]);

  return secondsRemaining;
};
