import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import type { ReactNode } from 'react';
import { DeckList } from './DeckList';
import { createMockDeck } from '../test/factories';

// Mock the tauri module
vi.mock('../lib/tauri', () => ({
  tauri: {
    listDecks: vi.fn(),
  },
}));

import { tauri } from '../lib/tauri';

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: 0,
      },
    },
  });

  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    );
  };
}

describe('DeckList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should show loading state', () => {
    vi.mocked(tauri.listDecks).mockImplementation(() => new Promise(() => {}));

    render(<DeckList />, { wrapper: createWrapper() });

    expect(screen.getByText('Loading decks...')).toBeInTheDocument();
  });

  it('should show error state with message', async () => {
    vi.mocked(tauri.listDecks).mockRejectedValue(new Error('Connection failed'));

    render(<DeckList />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByText(/Failed to load decks: Connection failed/)).toBeInTheDocument();
    });
  });

  it('should show empty state when no decks', async () => {
    vi.mocked(tauri.listDecks).mockResolvedValue([]);

    render(<DeckList />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'No decks yet' })).toBeInTheDocument();
      expect(
        screen.getByText('Import a markdown file or directory to create your first deck.')
      ).toBeInTheDocument();
    });
  });

  it('should render deck cards with stats', async () => {
    const decks = [
      createMockDeck({
        path: '/decks/spanish',
        name: 'Spanish Vocabulary',
        card_count: 100,
        new_count: 20,
        due_count: 15,
      }),
      createMockDeck({
        path: '/decks/math',
        name: 'Math Formulas',
        card_count: 50,
        new_count: 10,
        due_count: 5,
      }),
    ];
    vi.mocked(tauri.listDecks).mockResolvedValue(decks);

    render(<DeckList />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Your Decks' })).toBeInTheDocument();
    });

    // Check first deck
    expect(screen.getByText('Spanish Vocabulary')).toBeInTheDocument();
    expect(screen.getByText('100')).toBeInTheDocument();
    expect(screen.getByText('20')).toBeInTheDocument();
    expect(screen.getByText('15')).toBeInTheDocument();

    // Check second deck
    expect(screen.getByText('Math Formulas')).toBeInTheDocument();
    expect(screen.getByText('50')).toBeInTheDocument();
  });

  it('should link to study pages with encoded paths', async () => {
    const deck = createMockDeck({
      path: '/decks/my deck',
      name: 'My Deck',
    });
    vi.mocked(tauri.listDecks).mockResolvedValue([deck]);

    render(<DeckList />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByText('My Deck')).toBeInTheDocument();
    });

    const link = screen.getByRole('link', { name: /My Deck/ });
    expect(link).toHaveAttribute('href', '/study/%2Fdecks%2Fmy%20deck');
  });

  it('should render correct class names', async () => {
    const deck = createMockDeck();
    vi.mocked(tauri.listDecks).mockResolvedValue([deck]);

    const { container } = render(<DeckList />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(container.querySelector('.deck-list')).toBeInTheDocument();
    });

    expect(container.querySelector('.decks')).toBeInTheDocument();
    expect(container.querySelector('.deck-card')).toBeInTheDocument();
    expect(container.querySelector('.deck-name')).toBeInTheDocument();
    expect(container.querySelector('.deck-stats')).toBeInTheDocument();
  });

  it('should display stat labels', async () => {
    const deck = createMockDeck();
    vi.mocked(tauri.listDecks).mockResolvedValue([deck]);

    render(<DeckList />, { wrapper: createWrapper() });

    await waitFor(() => {
      expect(screen.getByText('cards')).toBeInTheDocument();
      expect(screen.getByText('new')).toBeInTheDocument();
      expect(screen.getByText('due')).toBeInTheDocument();
    });
  });
});
