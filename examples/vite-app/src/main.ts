import systemPrompt from './prompts/system.mds';
import reviewerPrompt from './prompts/reviewer.mds';
import v2Features from './prompts/v2-features.mds';
import { codePassthrough, escapedBraces, shadowingStress, emptyCollections } from './stress';

console.log('=== System Prompt ===');
console.log(systemPrompt);
console.log('\n=== Reviewer Prompt ===');
console.log(reviewerPrompt);
console.log('\n=== v0.2.0 Features ===');
console.log(v2Features);

console.log('\n=== Stress Test: Code Passthrough ===');
console.log(codePassthrough);
console.log('\n=== Stress Test: Escaped Braces ===');
console.log(escapedBraces);
console.log('\n=== Stress Test: Shadowing ===');
console.log(shadowingStress);
console.log('\n=== Stress Test: Empty Collections ===');
console.log(emptyCollections);

export { systemPrompt, reviewerPrompt, v2Features, codePassthrough, escapedBraces, shadowingStress, emptyCollections };
