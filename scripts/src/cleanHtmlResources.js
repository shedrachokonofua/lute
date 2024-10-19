const fs = require("fs");
const path = require("path");
const cleanHtmlContent = (htmlContent) => {
  // Remove extra whitespace (spaces, tabs, newlines) between tags and inside text content
  htmlContent = htmlContent.replace(/\s+/g, " "); // Replace multiple whitespace characters with a single space
  htmlContent = htmlContent.replace(/>\s+</g, "><"); // Remove whitespace between tags

  // Remove all <script>...</script> tags
  htmlContent = htmlContent.replace(/<script[\s\S]*?<\/script>/gi, "");

  // Remove all <style>...</style> tags
  htmlContent = htmlContent.replace(/<style[\s\S]*?<\/style>/gi, "");

  // Remove all <!--...--> comments
  htmlContent = htmlContent.replace(/<!--[\s\S]*?-->/gi, "");

  // Remove all style and onclick attributes from tags
  htmlContent = htmlContent.replace(/\s*(style|onclick)=["'][^"']*["']/gi, "");

  // Trim leading and trailing whitespace
  htmlContent = htmlContent.trim();

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

        // Clean the HTML content
        let cleanedContent = cleanHtmlContent(htmlContent);

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
