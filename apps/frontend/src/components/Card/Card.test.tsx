import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Card } from './Card';
import { createMockCard } from '../../test/factories';

describe('Card', () => {
  it('should always display the question', () => {
    const card = createMockCard({ question: 'What is 2 + 2?' });
    const onReveal = vi.fn();

    render(<Card card={card} revealed={false} onReveal={onReveal} />);

    expect(screen.getByText('Question')).toBeInTheDocument();
    expect(screen.getByText('What is 2 + 2?')).toBeInTheDocument();
  });

  it('should show reveal button when not revealed', () => {
    const card = createMockCard({ answer: '4' });
    const onReveal = vi.fn();

    render(<Card card={card} revealed={false} onReveal={onReveal} />);

    expect(screen.getByRole('button', { name: 'Show Answer' })).toBeInTheDocument();
    expect(screen.queryByText('4')).not.toBeInTheDocument();
  });

  it('should show answer when revealed', () => {
    const card = createMockCard({ answer: '4' });
    const onReveal = vi.fn();

    render(<Card card={card} revealed={true} onReveal={onReveal} />);

    expect(screen.getByText('Answer')).toBeInTheDocument();
    expect(screen.getByText('4')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Show Answer' })).not.toBeInTheDocument();
  });

  it('should call onReveal when button is clicked', async () => {
    const user = userEvent.setup();
    const card = createMockCard();
    const onReveal = vi.fn();

    render(<Card card={card} revealed={false} onReveal={onReveal} />);

    await user.click(screen.getByRole('button', { name: 'Show Answer' }));

    expect(onReveal).toHaveBeenCalledTimes(1);
  });

  it('should render with correct class names', () => {
    const card = createMockCard();
    const onReveal = vi.fn();

    const { container } = render(<Card card={card} revealed={false} onReveal={onReveal} />);

    expect(container.querySelector('.card')).toBeInTheDocument();
    expect(container.querySelector('.card-content')).toBeInTheDocument();
    expect(container.querySelector('.card-question')).toBeInTheDocument();
  });

  it('should display long questions correctly', () => {
    const longQuestion = 'What is the process by which plants convert sunlight into energy?';
    const card = createMockCard({ question: longQuestion });
    const onReveal = vi.fn();

    render(<Card card={card} revealed={false} onReveal={onReveal} />);

    expect(screen.getByText(longQuestion)).toBeInTheDocument();
  });

  it('should display long answers correctly when revealed', () => {
    const longAnswer = 'Photosynthesis is the process by which green plants and some other organisms use sunlight to synthesize foods from carbon dioxide and water.';
    const card = createMockCard({ answer: longAnswer });
    const onReveal = vi.fn();

    render(<Card card={card} revealed={true} onReveal={onReveal} />);

    expect(screen.getByText(longAnswer)).toBeInTheDocument();
  });
});
