import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { tauri } from '../lib/tauri';

export function DeckList() {
  const { data: decks, isLoading, error } = useQuery({
    queryKey: ['decks'],
    queryFn: tauri.listDecks,
  });

  if (isLoading) {
    return <div className="loading">Loading decks...</div>;
  }

  if (error) {
    return <div className="error">Failed to load decks: {(error as Error).message}</div>;
  }

  if (!decks || decks.length === 0) {
    return (
      <div className="empty-state">
        <h2>No decks yet</h2>
        <p>Import a markdown file or directory to create your first deck.</p>
      </div>
    );
  }

  return (
    <div className="deck-list">
      <h1>Your Decks</h1>

      <div className="decks">
        {decks.map((deck) => (
          <Link key={deck.path} to={`/study/${encodeURIComponent(deck.path)}`} className="deck-card">
            <h3 className="deck-name">{deck.name}</h3>
            <div className="deck-stats">
              <span className="stat">
                <span className="stat-value">{deck.card_count}</span>
                <span className="stat-label">cards</span>
              </span>
              <span className="stat stat-new">
                <span className="stat-value">{deck.new_count}</span>
                <span className="stat-label">new</span>
              </span>
              <span className="stat stat-due">
                <span className="stat-value">{deck.due_count}</span>
                <span className="stat-label">due</span>
              </span>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
