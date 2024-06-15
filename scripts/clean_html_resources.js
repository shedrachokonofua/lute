const fs = require("fs");
const path = require("path");

const removeScriptAndStyleTags = (htmlContent) => {
  // Remove all <script>...</script> tags
  htmlContent = htmlContent.replace(/<script[\s\S]*?<\/script>/gi, "");

  // Remove all <style>...</style> tags
  htmlContent = htmlContent.replace(/<style[\s\S]*?<\/style>/gi, "");

  return htmlContent;
};

const removeComments = (htmlContent) => {
  // Remove all <!--...--> comments
  htmlContent = htmlContent.replace(/<!--[\s\S]*?-->/gi, "");

  return htmlContent;
};

const removeWhitespace = (htmlContent) => {
  // Remove extra whitespace (spaces, tabs, newlines) between tags
  htmlContent = htmlContent.replace(/>[\s]+</g, "><");

  return htmlContent;
};

const removeStyleAndOnclickAttributes = (htmlContent) => {
  // Remove all style and onclick attributes from tags
  htmlContent = htmlContent.replace(/\s*(style|onclick)=["'][^"']*["']/gi, "");

  return htmlContent;
};

const cleanHtmlFilesInFolder = (folderPath) => {
  // Iterate over all files in the given folder
  fs.readdirSync(folderPath).forEach((filename) => {
    if (filename.endsWith(".html")) {
      const filePath = path.join(folderPath, filename);

      try {
        // Read the content from the HTML file
        let htmlContent = fs.readFileSync(filePath, "utf-8");

        // Remove <script> and <style> tags
        let cleanedContent = removeScriptAndStyleTags(htmlContent);

        // Remove comments
        cleanedContent = removeComments(cleanedContent);

        // Remove whitespace
        cleanedContent = removeWhitespace(cleanedContent);

        // Remove style and onclick attributes
        cleanedContent = removeStyleAndOnclickAttributes(cleanedContent);

        // Write the cleaned content back to the HTML file
        fs.writeFileSync(filePath, cleanedContent, "utf-8");

        console.log(`Cleaned ${filePath}`);
      } catch (e) {
        console.log(`Error cleaning ${filePath}: ${e}`);
      }
    }
  });
};

if (process.argv.length !== 3) {
  console.log("Usage: node remove_tags.js <folder_path>");
  process.exit(1);
}

const folderPath = process.argv[2];

if (!fs.existsSync(folderPath) || !fs.lstatSync(folderPath).isDirectory()) {
  console.log(`Error: ${folderPath} is not a directory`);
  process.exit(1);
}

// Clean HTML files in the given folder
cleanHtmlFilesInFolder(folderPath);
