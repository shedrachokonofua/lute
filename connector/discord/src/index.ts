import { Client, Message } from "discord.js";
import { config } from "./config";
import { getAlbumMonitor, recommendAlbums } from "./lute";
import OpenAI from "openai";
import { RunnableToolFunction } from "openai/lib/RunnableFunction";
import {
  ChatCompletionMessageParam,
  ChatCompletionSystemMessageParam,
} from "openai/resources";
import { lute } from "./proto/lute";

const openai = new OpenAI({
  apiKey: config.openai.key,
});

const client = new Client({
  intents: ["Guilds", "GuildMessages", "DirectMessages", "MessageContent"],
});

client.once("ready", () => {
  console.log("Discord bot is ready! 🤖");
});

client.on("interactionCreate", async (interaction) => {
  console.log(interaction);
});

client.login(config.discord.token);

const startingPrompt: ChatCompletionSystemMessageParam = {
  role: "system",
  content:
    "You are a chat front-end for a state of the art music recommendation engine. Prompt the user a bit but creatively to drill down into their preferences, without being too direct and explicits. Don't interrogate, Be engaging and succinct. Don't call functions until you are done gathering information. Call the complete function when you detect the user is done.",
};
class ChatSession {
  messages: ChatCompletionMessageParam[] = [startingPrompt];
  private genres: string[];
  private languages: string[];
  private descriptors: string[];

  constructor(albumMonitor: lute.AlbumMonitor) {
    this.genres = albumMonitor?.aggregatedGenres.map((genre) => genre.name);
    this.languages = albumMonitor?.aggregatedLanguages.map(
      (language) => language.name
    );
    this.descriptors = albumMonitor?.aggregatedDescriptors.map(
      (descriptor) => descriptor.name
    );
  }

  get hasStarted() {
    return this.messages.length > 1;
  }

  public async addUserMessage(message: string) {
    this.messages.push({
      role: "user",
      content: message,
    });
  }

  public async handleMessage(message: Message<boolean>) {
    this.addUserMessage(message.content.replace(/<@&?\d+>/g, "").trim());
    const runner = await openai.beta.chat.completions
      .runTools({
        model: "gpt-4o",
        tools: [
          {
            type: "function",
            function: {
              name: "recommendAlbums",
              description: "Recommend albums based on user specifications.",
              parameters: {
                type: "object",
                properties: {
                  includePrimaryGenres: {
                    description: "Primary genres to include in recommendations",
                    type: "array",
                    items: {
                      enum: this.genres,
                    },
                  },
                  includeSecondaryGenres: {
                    description:
                      "Secondary genres to include in recommendations",
                    type: "array",
                    items: {
                      enum: this.genres,
                    },
                  },
                  includeLanguages: {
                    description: "Languages to include in recommendations",
                    type: "array",
                    items: {
                      enum: this.languages,
                    },
                  },
                  excludeLanguages: {
                    description: "Languages to exclude in recommendations",
                    type: "array",
                    items: {
                      enum: this.languages,
                    },
                  },
                  includeDescriptors: {
                    description: "Descriptors to include in recommendations",
                    type: "array",
                    items: {
                      enum: this.descriptors,
                    },
                  },
                  excludeDescriptors: {
                    description: "Descriptors to exclude in recommendations",
                    type: "array",
                    items: {
                      enum: this.descriptors,
                    },
                  },
                  minReleaseYear: {
                    type: "number",
                    description: "Minimum release year for recommendations",
                  },
                  maxReleaseYear: {
                    type: "number",
                    description: "Maximum release year for recommendations",
                  },
                },
              },
              function: async (args: any) =>
                recommendAlbums({
                  profileId: "clean-main",
                  filters: args as any,
                  limit: 5,
                }),
              parse: JSON.parse,
            },
          } as RunnableToolFunction<any>,
        ],
        messages: this.messages,
      })
      .on("message", (message) => console.log(message))
      .on("functionCall", (functionCall) =>
        console.log("functionCall", functionCall)
      )
      .on("functionCallResult", (functionCallResult) =>
        console.log("functionCallResult", functionCallResult)
      )
      .on("chatCompletion", (completion) => {
        const choice = completion.choices[0];
        this.messages.push(choice.message);
        if (choice.message.content) {
          message.reply(choice.message.content);
        }
      });
    await runner.finalChatCompletion();
  }

  public reset() {
    this.messages = [startingPrompt];
  }
}

(async () => {
  const albumMonitor = (await getAlbumMonitor())?.monitor;
  let chatSession = new ChatSession(albumMonitor);

  client.on("messageCreate", async (message) => {
    if (message.author.bot) return;
    await chatSession.handleMessage(message);
  });
})();
