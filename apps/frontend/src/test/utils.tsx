import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, type RenderOptions } from '@testing-library/react';
import { MemoryRouter, type MemoryRouterProps } from 'react-router-dom';
import type { ReactElement, ReactNode } from 'react';

// Create a new QueryClient for each test to avoid shared state
function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
        staleTime: 0,
      },
      mutations: {
        retry: false,
      },
    },
  });
}

interface WrapperProps {
  children: ReactNode;
}

interface CustomRenderOptions extends Omit<RenderOptions, 'wrapper'> {
  routerProps?: MemoryRouterProps;
  queryClient?: QueryClient;
}

// Create a wrapper with all providers
function createWrapper(options: CustomRenderOptions = {}) {
  const queryClient = options.queryClient ?? createTestQueryClient();

  return function Wrapper({ children }: WrapperProps) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter {...options.routerProps}>{children}</MemoryRouter>
      </QueryClientProvider>
    );
  };
}

// Custom render function that includes providers
function customRender(ui: ReactElement, options: CustomRenderOptions = {}) {
  const { routerProps, queryClient, ...renderOptions } = options;

  return {
    ...render(ui, {
      wrapper: createWrapper({ routerProps, queryClient }),
      ...renderOptions,
    }),
    queryClient: queryClient ?? createTestQueryClient(),
  };
}

// Re-export everything from testing-library
export * from '@testing-library/react';
export { userEvent } from '@testing-library/user-event';

// Override render with custom render
export { customRender as render, createTestQueryClient };
