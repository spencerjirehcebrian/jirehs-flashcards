import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { StudyComplete } from './StudyComplete';

function renderWithRouter(component: React.ReactElement) {
  return render(<MemoryRouter>{component}</MemoryRouter>);
}

describe('StudyComplete', () => {
  it('should display completion message', () => {
    renderWithRouter(<StudyComplete />);

    expect(screen.getByRole('heading', { name: 'Session Complete' })).toBeInTheDocument();
    expect(
      screen.getByText("You've reviewed all cards for this session.")
    ).toBeInTheDocument();
  });

  it('should render restart button when callback provided', () => {
    const onRestart = vi.fn();

    renderWithRouter(<StudyComplete onRestart={onRestart} />);

    expect(screen.getByRole('button', { name: 'Study Again' })).toBeInTheDocument();
  });

  it('should hide restart button when callback undefined', () => {
    renderWithRouter(<StudyComplete />);

    expect(screen.queryByRole('button', { name: 'Study Again' })).not.toBeInTheDocument();
  });

  it('should call onRestart when restart button clicked', async () => {
    const user = userEvent.setup();
    const onRestart = vi.fn();

    renderWithRouter(<StudyComplete onRestart={onRestart} />);

    await user.click(screen.getByRole('button', { name: 'Study Again' }));

    expect(onRestart).toHaveBeenCalledTimes(1);
  });

  it('should always show back to decks link', () => {
    renderWithRouter(<StudyComplete />);

    const link = screen.getByRole('link', { name: 'Back to Decks' });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute('href', '/');
  });

  it('should render with correct class names', () => {
    const { container } = renderWithRouter(<StudyComplete />);

    expect(container.querySelector('.study-complete')).toBeInTheDocument();
    expect(container.querySelector('.study-complete-actions')).toBeInTheDocument();
  });

  it('should show both buttons when onRestart is provided', () => {
    const onRestart = vi.fn();

    renderWithRouter(<StudyComplete onRestart={onRestart} />);

    expect(screen.getByRole('button', { name: 'Study Again' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Back to Decks' })).toBeInTheDocument();
  });
});
