/***********************************************************************************************************************
 * Copyright (C) 2016 Andrew Zonenberg and contributors                                                                *
 *                                                                                                                     *
 * This program is free software; you can redistribute it and/or modify it under the terms of the GNU Lesser General   *
 * Public License as published by the Free Software Foundation; either version 2.1 of the License, or (at your option) *
 * any later version.                                                                                                  *
 *                                                                                                                     *
 * This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied  *
 * warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Lesser General Public License for     *
 * more details.                                                                                                       *
 *                                                                                                                     *
 * You should have received a copy of the GNU Lesser General Public License along with this program; if not, you may   *
 * find one here:                                                                                                      *
 * https://www.gnu.org/licenses/old-licenses/lgpl-2.1.txt                                                              *
 * or you may search the http://www.gnu.org website for the version 2.1 license, or you may write to the Free Software *
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA                                      *
 **********************************************************************************************************************/

#include <log.h>
#include <Greenpak4.h>

using namespace std;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Construction / destruction

Greenpak4MuxedClockBuffer::Greenpak4MuxedClockBuffer(
	Greenpak4Device* device,
	unsigned int bufnum,
	unsigned int matrix,
	unsigned int cbase)
	: Greenpak4ClockBuffer(device, bufnum, matrix, -1, cbase)
{
}

Greenpak4MuxedClockBuffer::~Greenpak4MuxedClockBuffer()
{

}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Serialization

bool Greenpak4MuxedClockBuffer::Load(bool* bitstream)
{
	unsigned int muxsel = (bitstream[m_configBase + 1] << 1) | bitstream[m_configBase + 0];
	for(auto it : m_inputs)
	{
		if(it.second == muxsel)
		{
			m_input = it.first;
			break;
		}
	}

	return true;
}

bool Greenpak4MuxedClockBuffer::Save(bool* bitstream)
{
	////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
	// INPUT BUS

	//none

	////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
	// CONFIGURATION

	//Grounded input is legal even if not a valid muxsel
	if(m_input.IsPowerRail() && !m_input.GetPowerRailValue())
		return true;

	if(m_inputs.find(m_input) == m_inputs.end())
	{
		LogError("Greenpak4MuxedClockBuffer: invalid input\n");
		return false;
	}

	unsigned int muxsel = m_inputs[m_input];

	bitstream[m_configBase + 0] = (muxsel & 1) ? true : false;
	bitstream[m_configBase + 1] = (muxsel & 2) ? true : false;

	return true;
}
