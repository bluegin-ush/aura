import { useState } from 'react'
import { Bike } from 'lucide-react'
import './App.css'
import Dashboard from './components/Dashboard'
import Parts from './components/Parts'
import Motos from './components/Motos'
import Orders from './components/Orders'

function App() {
  const [section, setSection] = useState('dashboard')

  return (
    <>
      <nav>
        <div className="logo">
          <Bike size={28} /> MotoStock
        </div>
        <ul>
          <li><a href="#" onClick={(e) => { e.preventDefault(); setSection('dashboard'); }} className={section === 'dashboard' ? 'active' : ''}>Dashboard</a></li>
          <li><a href="#" onClick={(e) => { e.preventDefault(); setSection('parts'); }} className={section === 'parts' ? 'active' : ''}>Repuestos</a></li>
          <li><a href="#" onClick={(e) => { e.preventDefault(); setSection('motos'); }} className={section === 'motos' ? 'active' : ''}>Motos</a></li>
          <li><a href="#" onClick={(e) => { e.preventDefault(); setSection('orders'); }} className={section === 'orders' ? 'active' : ''}>Órdenes</a></li>
        </ul>
      </nav>

      <main>
        {section === 'dashboard' && <Dashboard onNavigate={setSection} />}
        {section === 'parts' && <Parts />}
        {section === 'motos' && <Motos />}
        {section === 'orders' && <Orders />}
      </main>
    </>
  )
}

export default App
