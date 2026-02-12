<script setup lang="ts">
import MarkdownIt from "markdown-it";
import { computed } from "vue";

const props = defineProps<{
  content: string;
}>();

const md = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
});

const rendered = computed(() => {
  return md.render(props.content);
});
</script>

<template>
  <div class="markdown-content" v-html="rendered" />
</template>

<style scoped>
.markdown-content {
  word-wrap: break-word;
  overflow-wrap: break-word;
}

.markdown-content :deep(*) {
  margin: 0;
}

.markdown-content :deep(h1) {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.75rem;
  color: hsl(var(--foreground));
}

.markdown-content :deep(h2) {
  font-size: 1.125rem;
  font-weight: 600;
  margin-top: 1rem;
  margin-bottom: 0.5rem;
  color: hsl(var(--foreground));
}

.markdown-content :deep(h3) {
  font-size: 1rem;
  font-weight: 600;
  margin-top: 0.75rem;
  margin-bottom: 0.5rem;
  color: hsl(var(--foreground));
}

.markdown-content :deep(p) {
  margin-bottom: 0.5rem;
  line-height: 1.5;
  color: hsl(var(--muted-foreground));
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin-bottom: 0.5rem;
  padding-left: 1.25rem;
  color: hsl(var(--muted-foreground));
}

.markdown-content :deep(li) {
  margin-bottom: 0.25rem;
}

.markdown-content :deep(code) {
  font-size: 0.875rem;
  padding: 0.125rem 0.375rem;
  background-color: hsl(var(--muted));
  border-radius: calc(var(--radius) - 2px);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  word-break: break-all;
}

.markdown-content :deep(pre) {
  background-color: hsl(var(--muted));
  padding: 0.75rem;
  border-radius: var(--radius);
  overflow-x: auto;
  margin-bottom: 0.5rem;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.markdown-content :deep(pre code) {
  background-color: transparent;
  padding: 0;
  word-break: normal;
}

.markdown-content :deep(a) {
  color: hsl(var(--primary));
  text-decoration: none;
  word-break: break-all;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(strong) {
  font-weight: 600;
  color: hsl(var(--foreground));
}

.markdown-content :deep(blockquote) {
  border-left: 3px solid hsl(var(--border));
  padding-left: 0.75rem;
  margin-left: 0;
  margin-bottom: 0.5rem;
  color: hsl(var(--muted-foreground));
}

.markdown-content :deep(hr) {
  border: none;
  border-top: 1px solid hsl(var(--border));
  margin: 0.75rem 0;
}
</style>
