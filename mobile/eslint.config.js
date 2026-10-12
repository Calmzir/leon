// Lint rules: Expo's defaults. The generated bindings are not linted.
const { defineConfig } = require('eslint/config');
const expo = require('eslint-config-expo/flat');

module.exports = defineConfig([
  expo,
  { ignores: ['dist/*', 'android/*', 'ios/*', '.expo/*', 'modules/*', 'scripts/*'] },
]);
