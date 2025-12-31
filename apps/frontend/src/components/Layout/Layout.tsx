import { Link, Outlet, useLocation } from 'react-router-dom';

export function Layout() {
  const location = useLocation();

  return (
    <div className="app">
      <header className="header">
        <Link to="/" className="logo">
          Jireh's Flashcards
        </Link>
        <nav className="nav">
          <Link to="/" className={location.pathname === '/' ? 'active' : ''}>
            Decks
          </Link>
          <Link to="/stats" className={location.pathname.startsWith('/stats') ? 'active' : ''}>
            Stats
          </Link>
          <Link to="/settings" className={location.pathname === '/settings' ? 'active' : ''}>
            Settings
          </Link>
        </nav>
      </header>

      <main className="main">
        <Outlet />
      </main>
    </div>
  );
}
