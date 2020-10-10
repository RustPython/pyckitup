const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const { CleanWebpackPlugin } = require("clean-webpack-plugin");

module.exports = {
  entry: "./pyckitup.js",
  output: {
    filename: "pyckitup.js",
  },
  mode: "production",
  plugins: [
    new WasmPackPlugin({
      crateDirectory: __dirname,
      forceMode: "release",
    }),
    new CleanWebpackPlugin(),
  ],
};
