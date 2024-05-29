import dotenv from "dotenv";

dotenv.config();

const getStringEnv = (key: string, defaultValue?: string): string => {
  const value = process.env[key];
  if (value === undefined) {
    if (defaultValue === undefined) {
      throw new Error(`Missing required environment variable: ${key}`);
    }
    return defaultValue;
  }
  return value;
};

export const config = {
  coreUrl: getStringEnv("CORE_URL"),
  discord: {
    token: getStringEnv("DISCORD_TOKEN"),
  },
  openai: {
    key: getStringEnv("OPENAI_API_KEY"),
  },
};
