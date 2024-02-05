const path = require('path');

const CopyPlugin = require('copy-webpack-plugin')
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');

const dist = path.resolve(__dirname, 'dist');

const baseConfig = {
  mode: 'production',

  entry: {
    banyanfs: './js/banyanfs.js',
  },

  resolve: {
    extensions: ['.js', '.wasm'],
  },

  plugins: [
    new WasmPackPlugin({
      crateDirectory: __dirname,
      extraArgs: '--dev',
      outName: 'banyanfs',
    }),
  ],

  experiments: {
    asyncWebAssembly: true,
  },
}

const devServerConfig = {
  devServer: {
    host: '127.0.0.1',
    port: 8000,
  },
};

const outputConfig = {
  output: {
    path: dist,
    filename: '[name].js',
  }
}

module.exports = (env) => {
  return [{
    ...baseConfig,
    ...devServerConfig,
    ...outputConfig,
  }];
};
