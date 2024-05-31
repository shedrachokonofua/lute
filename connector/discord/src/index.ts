import { Client, Message } from "discord.js";
import { config } from "./config";
import {
  findSimilarAlbums,
  getAlbumMonitor,
  recommendAlbums,
  searchAlbums,
} from "./lute";
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
	"You are a chat front-end for a state-of-the-art music recommendation engine called Lute. Creatively drill down into the user's preferences, without being too direct and explicit. Don't interrogate, Be engaging and brief. You're here to help them discover new music, be suggestive but not imposing. When telling them about an album, be engaging and brief, don't include cover images as they maybe dead. Tell them about your capabilities to start. Extrapolate what they mean, enrich requests as you see fit for best results. Responses must be less than 2000 characters. Steer away from your own knowledge of music for recommendations, be a conduit for Lute, only use knowledge to help user prompt and make the responses engaging. Liberally call the functions to gather as much context as you need. Lute is very powerful.",
};

function chunkText(text: string, maxChunkLength: number): string[] {
  const chunks = [];
  let chunk = "";
  for (const word of text.split(" ")) {
    if (chunk.length + word.length > maxChunkLength) {
      chunks.push(chunk);
      chunk = "";
    }
    chunk += word + " ";
  }
  chunks.push(chunk);
  return chunks;
}
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
    if (message.content.match(/<@&?\d+>/) || message.content === "reset") {
      this.reset();
      return;
    }
    this.addUserMessage(message.content.replace(/<@&?\d+>/g, "").trim());
    const runner = await openai.beta.chat.completions
      .runTools({
        model: "gpt-4o",
        tools: [
          {
            type: "function",
            function: {
              name: "recommendAlbums",
              description:
                "Recommend albums based on user specifications. Only use this to recommend albums, not to search for albums. Use searchAlbum for that. Have a specific intent in mind when using this.",
              parameters: {
                type: "object",
                properties: {
                  includePrimaryGenres: {
                    description: "Primary genres to include in recommendations",
                    $ref: "#/definitions/genreArray",
                  },
                  includeSecondaryGenres: {
                    description:
                      "Secondary genres to include in recommendations",
                    $ref: "#/definitions/genreArray",
                  },
                  includeLanguages: {
                    description: "Languages to include in recommendations",
                    $ref: "#/definitions/languageArray",
                  },
                  excludeLanguages: {
                    description: "Languages to exclude in recommendations",
                    $ref: "#/definitions/languageArray",
                  },
                  includeDescriptors: {
                    description: "Descriptors to include in recommendations",
                    $ref: "#/definitions/descriptorArray",
                  },
                  excludeDescriptors: {
                    description: "Descriptors to exclude in recommendations",
                    $ref: "#/definitions/descriptorArray",
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
                definitions: {
                  genreArray: {
                    type: "array",
                    items: {
                      enum: this.genres,
                    },
                  },
                  languageArray: {
                    type: "array",
                    items: {
                      enum: this.languages,
                    },
                  },
                  descriptorArray: {
                    type: "array",
                    items: {
                      enum: this.descriptors,
                    },
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
          {
            type: "function",
            function: {
              name: "searchAlbum",
              description:
                "Search album based on user query. You can use this to search up for a specific album or artist's albums, but don't rely on this for recommendations, but use this to build more context. Don't use this to determine good/bad. You can also use this to get the fileName of the album to use in other functions.",
              parameters: {
                type: "object",
                properties: {
                  pagination: {
                    type: "object",
                    properties: {
                      offset: {
                        type: "number",
                        description: "Offset for search results",
                      },
                      limit: {
                        type: "number",
                        description:
                          "Number of results to return. Be liberal with this for artist you think have a lot of albums.",
                      },
                    },
                  },
                  query: {
                    type: "object",
                    properties: {
                      text: {
                        type: "string",
                        description:
                          "Text to search for. Can ONLY be an album name or artist name. Don't put genres, languages or descriptors here, it will be incorrect.",
                      },
                      excludeFileNames: {
                        type: "array",
                        items: {
                          type: "string",
                        },
                        description:
                          "File names to exclude from search results.",
                      },
                      includePrimaryGenres: {
                        description:
                          "Primary genres to include in recommendations",
                        $ref: "#/definitions/genreArray",
                      },
                      includeSecondaryGenres: {
                        description:
                          "Secondary genres to include in recommendations",
                        $ref: "#/definitions/genreArray",
                      },
                      includeLanguages: {
                        description: "Languages to include in recommendations",
                        $ref: "#/definitions/languageArray",
                      },
                      excludeLanguages: {
                        description: "Languages to exclude in recommendations",
                        $ref: "#/definitions/languageArray",
                      },
                      includeDescriptors: {
                        description:
                          "Descriptors to include in recommendations",
                        $ref: "#/definitions/descriptorArray",
                      },
                      excludeDescriptors: {
                        description:
                          "Descriptors to exclude in recommendations",
                        $ref: "#/definitions/descriptorArray",
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
                },
                required: ["query.text"],
              },
              function: async (args: any) => searchAlbums(args),
              parse: JSON.parse,
            },
          } as RunnableToolFunction<any>,
          {
            type: "function",
            function: {
              name: "findSimilarAlbums",
              description: "Find similar albums based on a given album.",
              parameters: {
                type: "object",
                properties: {
                  fileName: {
                    type: "string",
                    description:
                      "Lute album fileName to find similar albums for. This is the primary key for an album, so don't use this to search for an album you don't know the fileName for.",
                  },
                  limit: {
                    type: "number",
                    description: "Number of results to return.",
                  },
                  filters: {
                    type: "object",
                    properties: {
                      text: {
                        type: "string",
                        description:
                          "Text to search for. Can be an album name or artist name.",
                      },
                      excludeFileNames: {
                        type: "array",
                        items: {
                          type: "string",
                        },
                        description:
                          "File names to exclude from search results.",
                      },
                      includePrimaryGenres: {
                        description:
                          "Primary genres to include in recommendations",
                        $ref: "#/definitions/genreArray",
                      },
                      includeSecondaryGenres: {
                        description:
                          "Secondary genres to include in recommendations",
                        $ref: "#/definitions/genreArray",
                      },
                      includeLanguages: {
                        description: "Languages to include in recommendations",
                        $ref: "#/definitions/languageArray",
                      },
                      excludeLanguages: {
                        description: "Languages to exclude in recommendations",
                        $ref: "#/definitions/languageArray",
                      },
                      includeDescriptors: {
                        description:
                          "Descriptors to include in recommendations",
                        $ref: "#/definitions/descriptorArray",
                      },
                      excludeDescriptors: {
                        description:
                          "Descriptors to exclude in recommendations",
                        $ref: "#/definitions/descriptorArray",
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
                },
                required: ["fileName"],
              },
              function: async (args: any) => findSimilarAlbums(args),
              parse: JSON.parse,
            },
          } as RunnableToolFunction<any>,
        ],
        messages: this.messages,
      })
      .on("functionCall", (functionCall) =>
        console.log("functionCall", functionCall)
      )
      .on("functionCallResult", (functionCallResult) =>
        console.log("functionCallResult")
      )
      .on("chatCompletion", async (completion) => {
        const content = completion.choices?.[0]?.message?.content;
        if (content) {
          for (const chunk of chunkText(content, 2000)) {
            await message.reply(chunk);
          }
        }
      });
    await runner.finalChatCompletion();
    console.log(runner.messages);
    this.messages = runner.messages;
  }

  public reset() {
    this.messages = [startingPrompt];
  }
}

(async () => {
  const albumMonitor = await getAlbumMonitor();
  let chatSession = new ChatSession(albumMonitor);

  client.on("messageCreate", async (message) => {
    if (message.author.bot) return;
    await chatSession.handleMessage(message);
  });
})();
