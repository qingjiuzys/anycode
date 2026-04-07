import DefaultTheme from 'vitepress/theme'
import './style.css'

export default {
  extends: DefaultTheme,
  enhanceApp(ctx) {
    DefaultTheme.enhanceApp?.(ctx)

    ctx.router.onAfterRouteChange = async () => {
      if (typeof window === 'undefined') return

      requestAnimationFrame(() => {
        const openedFlyouts = document.querySelectorAll<HTMLButtonElement>(
          '.VPFlyout .button[aria-expanded="true"]'
        )

        for (const button of openedFlyouts) {
          button.click()
        }

        if (document.activeElement instanceof HTMLElement) {
          document.activeElement.blur()
        }
      })
    }
  }
}
