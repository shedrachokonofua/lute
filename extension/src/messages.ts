import { PageContextValue } from "./types";

class MessageFactory<Request, Response> {
  constructor(public readonly type: string) {}
  buildRequest(data: Request): { type: string; payload: Request } {
    return {
      type: this.type,
      payload: data,
    };
  }
  send(data: Request): Promise<Response> {
    return new Promise((resolve, reject) => {
      chrome.runtime.sendMessage(this.buildRequest(data), (response) => {
        if (response === undefined) {
          reject("No response");
        } else {
          resolve(response.data);
        }
      });
    });
  }
  listen(handler: (data: Request) => Promise<Response>) {
    chrome.runtime.onMessage.addListener((request, _, sendResponse) => {
      if (request.type === this.type) {
        handler(request.payload).then((response) => {
          sendResponse({
            data: response,
          });
        });
        return true;
      }
    });
  }
}

export const getPageContextMessage = new MessageFactory<
  { url: string },
  PageContextValue | undefined
>("get-page-context");
