import solanaConfig from '@solana/eslint-config-solana';
import { defineConfig } from 'eslint/config';

export default defineConfig([
    {
        files: ['packages/**/*.ts', 'packages/**/*.(c|m)?js'],
        extends: [solanaConfig],
    },
]);