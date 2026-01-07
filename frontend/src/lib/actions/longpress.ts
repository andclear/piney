export function longpress(node: HTMLElement, duration = 500) {
    let timer: ReturnType<typeof setTimeout>;

    const handleStart = (e: TouchEvent | MouseEvent) => {
        timer = setTimeout(() => {
            node.dispatchEvent(new CustomEvent("longpress", { detail: { originalEvent: e } }));
        }, duration);
    };

    const handleEnd = () => {
        clearTimeout(timer);
    };

    node.addEventListener("mousedown", handleStart);
    node.addEventListener("touchstart", handleStart, { passive: true });
    node.addEventListener("mouseup", handleEnd);
    node.addEventListener("mouseleave", handleEnd);
    node.addEventListener("touchend", handleEnd);
    node.addEventListener("touchcancel", handleEnd);

    return {
        destroy() {
            node.removeEventListener("mousedown", handleStart);
            node.removeEventListener("touchstart", handleStart);
            node.removeEventListener("mouseup", handleEnd);
            node.removeEventListener("mouseleave", handleEnd);
            node.removeEventListener("touchend", handleEnd);
            node.removeEventListener("touchcancel", handleEnd);
            clearTimeout(timer);
        },
    };
}
