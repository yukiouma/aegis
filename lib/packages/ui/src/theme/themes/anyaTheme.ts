import { createTheme } from '@mui/material/styles';

export const anyaTheme = createTheme({
  palette: {
    mode: 'light',
    primary: {
      main: '#fb96aa', // Anya's Hair (Strawberry Pink)
      light: '#FFD1DC',
      dark: '#E59FB0',
      contrastText: '#5C3D45',
    },
    secondary: {
      main: '#D4AF37', // Stella Star Gold
    },
    info: {
      main: '#AEEEEE', // Telepathy Sparkle (Mint Blue)
    },
    background: {
      default: '#FFF9F1', // Warm Retro 1960s Cream
      paper: '#FFFFFF',
    },
    text: {
      primary: '#5f2b2b', // Deep Eden Academy Charcoal
      secondary: '#E59FB0', // Muted Pink
    },
  },
  shape: {
    borderRadius: 16, // Rounded and friendly like Anya's expressions
  },
  typography: {
    // Playful yet clean font choice
    fontFamily: '"Quicksand", "Inter", sans-serif',
    h1: {
      fontWeight: 700,
      color: '#5C3D45',
      fontFamily: '"Grandstander", cursive'
    },
    button: {
      textTransform: 'none',
      fontWeight: 700,
      fontSize: '1rem',
    },
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          padding: '8px 24px',
          boxShadow: '0 4px 0px #E59FB0', // Flat "Cartoon" shadow
          '&:hover': {
            transform: 'translateY(2px)',
            boxShadow: '0 2px 0px #E59FB0',
            backgroundColor: '#FFD1DC',
          },
        },
        containedSecondary: {
          boxShadow: '0 4px 0px #B8860B',
          '&:hover': {
            boxShadow: '0 2px 0px #B8860B',
          },
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          border: '2px solid #FFD1DC',
          boxShadow: '8px 8px 0px rgba(229, 159, 176, 0.1)', // Soft pink "drop" shadow
          position: 'relative',
          overflow: 'visible',
          // Decorative "Peanut" or "Stella Star" accent via pseudo-element
        },
      },
    },
    MuiAppBar: {
      styleOverrides: {
        root: {
          backgroundColor: '#2D2D2D', // Eden Academy Uniform Black
          color: '#D4AF37', // Gold trim
          borderBottom: '4px solid #D4AF37',
        },
      },
    },
    MuiCssBaseline: {
      styleOverrides: `
        body {
          /* Subtle dot pattern like a 60s comic book or newsprint */
          background-image: radial-gradient(#FFD1DC 1px, transparent 1px);
          background-size: 20px 20px;
        }
      `,
    },
  },
});
