import { onMounted, onUnmounted, ref } from "vue";

/** `true` quando o sistema pede movimento reduzido (`prefers-reduced-motion`). */
export function usePrefersReducedMotion() {
  const reduced = ref(false);
  let query: MediaQueryList | null = null;
  const update = () => {
    reduced.value = query?.matches ?? false;
  };

  onMounted(() => {
    if (typeof window.matchMedia !== "function") return;
    query = window.matchMedia("(prefers-reduced-motion: reduce)");
    update();
    query.addEventListener("change", update);
  });
  onUnmounted(() => query?.removeEventListener("change", update));

  return { reduced };
}
