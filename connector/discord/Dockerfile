FROM node:lts-buster-slim

# Create app directory
WORKDIR /usr/src/app

# Install app dependencies
COPY connector/discord/package*.json ./
RUN npm install

# Bundle app source
COPY connector/discord/ ./

# Build the TypeScript code
RUN npm run build

# Run the app
CMD [ "npm", "start" ]