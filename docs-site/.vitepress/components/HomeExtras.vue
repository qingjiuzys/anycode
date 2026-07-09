<script setup lang="ts">
const releasesUrl = 'https://github.com/qingjiuzys/anycode/releases'
const portalUrl = 'https://anycode.work'
</script>

<template>
  <section class="home-extras">
    <div class="home-extras__block">
      <h2>Download anyCode Desktop</h2>
      <p>macOS DMG from GitHub Releases. Windows/Linux builds available via workflow dispatch.</p>
      <div class="home-extras__cta-row">
        <a class="vp-button brand" :href="releasesUrl" target="_blank" rel="noreferrer">Download for macOS</a>
        <a class="vp-button alt" :href="portalUrl" target="_blank" rel="noreferrer">Cloud account</a>
      </div>
    </div>

    <div class="home-extras__block">
      <h2>Quick start</h2>
      <ol class="home-extras__steps">
        <li>Install anyCode.app and open Workbench</li>
        <li>Link cloud account or configure BYOK models in Settings</li>
        <li>Pick a scenario on Home — Code, Slides, Video, or Novel</li>
        <li>Ship artifacts from the workbench Artifacts panel</li>
      </ol>
    </div>

    <div class="home-extras__block home-extras__grid">
      <article>
        <h3>Code</h3>
        <p>Flutter / Web templates, agents, tools, and gates.</p>
      </article>
      <article>
        <h3>Slides</h3>
        <p><code>office-pptx</code> skill generates .pptx from outlines.</p>
      </article>
      <article>
        <h3>Video</h3>
        <p><code>video-script</code> skill for scripts + media assets.</p>
      </article>
      <article>
        <h3>Novel</h3>
        <p><code>novel-writer</code> skill for chapter files and continuation.</p>
      </article>
    </div>

    <div class="home-extras__block">
      <h2>FAQ</h2>
      <details>
        <summary>BYOK vs Cloud models?</summary>
        <p>Bring your own API keys locally, or link an anycode.work account for hosted models (Cloud Auto + named models).</p>
      </details>
      <details>
        <summary>WeChat bridge privacy?</summary>
        <p>Personal bridge uses read-only DB snapshots; credentials stay on your machine.</p>
      </details>
      <details>
        <summary>Where is data stored?</summary>
        <p><code>~/.anycode/config.json</code>, <code>projects.db</code>, and workspace project folders. Back up from Settings → Data.</p>
      </details>
    </div>
  </section>
</template>

<style scoped>
.home-extras {
  max-width: 960px;
  margin: 0 auto 4rem;
  padding: 0 24px;
  display: flex;
  flex-direction: column;
  gap: 2.5rem;
}
.home-extras__block h2 {
  margin: 0 0 0.75rem;
  font-size: 1.5rem;
}
.home-extras__block p,
.home-extras__block li {
  color: var(--vp-c-text-2);
  line-height: 1.6;
}
.home-extras__cta-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 1rem;
}
.home-extras__steps {
  margin: 0;
  padding-left: 1.25rem;
}
.home-extras__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
}
.home-extras__grid article {
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
  padding: 1rem;
}
.home-extras__grid h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
}
details {
  border-top: 1px solid var(--vp-c-divider);
  padding: 0.75rem 0;
}
details summary {
  cursor: pointer;
  font-weight: 600;
}
</style>
