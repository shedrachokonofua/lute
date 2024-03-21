import { NavigateOptions, useNavigate } from "react-router-dom";

export const getUpdatedQueryString = (updates: Record<string, any>) => {
  const url = new URL(window.location.href);
  const searchParams = new URLSearchParams(url.search);
  for (const [key, value] of Object.entries(updates)) {
    if (value !== undefined) {
      searchParams.set(key, value);
    }
  }
  return "?" + searchParams.toString();
};

export const useUpdateSearchParams = () => {
  const navigate = useNavigate();

  return (updates: Record<string, any>, options?: NavigateOptions) => {
    navigate(getUpdatedQueryString(updates), options);
  };
};
