"use client"
import { AllFilterContext } from "@/context/all-contex-provider"
import type React from "react"

import { routes } from "@/lib/route"
import ChevronDown from "@/svg/chevron-down"
import PlusIcon from "@/svg/plus-icon"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useContext, useState } from "react"

function DashboardRoot({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  const { setGetType, getType } = useContext(AllFilterContext)
  const [isOpen, setIsOpen] = useState(false)
  const router = useRouter()

  const tabs = [
    { label: "all pools", value: "all_pool" },
    { label: "active", value: "active" },
    { label: "locked", value: "locked" },
    { label: "closed", value: "closed" },
  ]

  interface TabProps {
    label: string
    value: string
    isActive: boolean
    onClick: (value: string) => void
  }

  const Tab = ({ label, value, isActive, onClick }: TabProps) => (
    <button
      className={`px-4 py-2.5 min-w-[100px] xl:min-w-[120px] rounded-t-full grid place-content-center capitalize cursor-pointer transition-colors ${
        isActive ? "bg-[#FFFFFF75]" : "bg-[#373737]"
      }`}
      onClick={() => onClick(value)}
    >
      {label}
    </button>
  )

  // Handle dropdown selection
  const handleSelectChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value
    if (value === "profile") {
      router.push(routes.profile)
    } else {
      setGetType(value)
    }
  }

  return (
    <div className="max-w-7xl mx-auto px-4 md:px-6 xl:px-14">
      <div className="flex justify-between items-center md:border-b md:border-[#373737] mb-6 md:mb-8 xl:mb-12">
        <div className="relative md:hidden border-[#373737] border rounded-full h-[30px] w-[150px] px-2">
          <select
            value={getType}
            onChange={handleSelectChange}
            className="border-none bg-inherit w-full absolute z-10 profile-select capitalize outline-none"
            onMouseDown={() => setIsOpen((prev) => !prev)}
          >
            {tabs.map((tab) => (
              <option key={tab.value} value={tab.value} className="bg-[#373737]">
                {tab.label}
              </option>
            ))}
            <option value="profile" className="bg-[#373737]">
              Profile
            </option>
          </select>
          <span className={`${isOpen ? "-rotate-180" : "rotate-0"} transition-all duration-500 absolute right-3 top-2`}>
            <ChevronDown />
          </span>
        </div>
        <div className="hidden md:flex justify-between items-center gap-3 capitalize">
          {tabs.map((tab) => (
            <Tab
              key={tab.value}
              label={tab.label}
              value={tab.value}
              isActive={getType === tab.value}
              onClick={setGetType}
            />
          ))}
          <div className="bg-[#373737] min-w-[100px] xl:min-w-[120px] px-4 py-2.5 grid place-content-center rounded-t-full">
            <Link href={routes.profile}>Profile</Link>
          </div>
        </div>
        <Link href={routes.createPool} className="flex gap-3 items-center md:mb-3">
          Create New Pool <PlusIcon />
        </Link>
      </div>
      {children}
    </div>
  )
}
export default DashboardRoot

