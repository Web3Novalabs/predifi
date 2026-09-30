import { renderHook, act } from '@testing-library/react';
import { useContainerSize } from '../hooks/useContainerSize';

// jsdom does not implement ResizeObserver, so provide a minimal mock that
// lets tests manually trigger the observer callback.
type ResizeObserverCallback = (entries: ResizeObserverEntry[], observer: ResizeObserver) => void;

let observerCallback: ResizeObserverCallback | null = null;
let observedElement: Element | null = null;

class MockResizeObserver implements ResizeObserver {
  constructor(callback: ResizeObserverCallback) {
    observerCallback = callback;
  }

  observe(target: Element): void {
    observedElement = target;
  }

  unobserve(): void {}

  disconnect(): void {
    observerCallback = null;
    observedElement = null;
  }
}

beforeAll(() => {
  // @ts-expect-error assigning mock to global for jsdom environment
  global.ResizeObserver = MockResizeObserver;
});

afterEach(() => {
  observerCallback = null;
  observedElement = null;
});

describe('useContainerSize', () => {
  it('returns the initial container size', () => {
    const element = document.createElement('div');
    Object.defineProperty(element, 'getBoundingClientRect', {
      configurable: true,
      value: () => ({
        width: 320,
        height: 240,
        top: 0,
        left: 0,
        right: 320,
        bottom: 240,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }),
    });

    const ref = { current: element };
    const { result } = renderHook(() => useContainerSize(ref));

    expect(result.current).toEqual({ width: 320, height: 240 });
  });

  it('updates the returned size when the observer fires', () => {
    const element = document.createElement('div');
    Object.defineProperty(element, 'getBoundingClientRect', {
      configurable: true,
      value: () => ({
        width: 320,
        height: 240,
        top: 0,
        left: 0,
        right: 320,
        bottom: 240,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }),
    });

    const ref = { current: element };
    const { result } = renderHook(() => useContainerSize(ref));

    expect(result.current).toEqual({ width: 320, height: 240 });

    // Simulate a resize by changing the measured size and firing the observer.
    Object.defineProperty(element, 'getBoundingClientRect', {
      configurable: true,
      value: () => ({
        width: 640,
        height: 480,
        top: 0,
        left: 0,
        right: 640,
        bottom: 480,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }),
    });

    act(() => {
      observerCallback?.(
        [
          {
            target: element,
            contentRect: element.getBoundingClientRect(),
          } as unknown as ResizeObserverEntry,
        ],
        {} as ResizeObserver,
      );
    });

    expect(observedElement).toBe(element);
    expect(result.current).toEqual({ width: 640, height: 480 });
  });
});
